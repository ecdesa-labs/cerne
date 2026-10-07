use cerne::application::{Command, CommandRun, OutboxPolicyProcessor, Query, TransactionalPorts};
use cerne::sqlite::SqliteDatabase;
use cerne::{DomainError, Error};
use rde::application::commands::accept_transfer::AcceptTransferCommand;
use rde::application::commands::cancel_transfer::CancelTransferCommand;
use rde::application::commands::create_transfer::CreateTransferCommand;
use rde::application::commands::reject_transfer::RejectTransferCommand;
use rde::application::commands::send_cancellation::SendCancellationCommand;
use rde::application::queries::pending_transfers::PendingTransfersQuery;
use rde::application::read_models::pending_transfers::PendingTransfer;
use rde::domain::entities::transfer::TransferStatus;
use rde::domain::services::units::WEI_PER_RDEC;
use rde::domain::value_objects::signed_transaction::SignedTransaction;
use rde::domain::value_objects::tx_hash::TxHash;
use rde::infrastructure::in_memory_blockchain::InMemoryBlockchain;
use rde::infrastructure::in_memory_kyc_registry::InMemoryKycRegistry;
use rde::infrastructure::in_memory_notifier::{InMemoryNotifier, Notification};
use rde::infrastructure::local_wallet::LocalWallet;
use rde::ports::{ExternalSystems, Ports, command_registry};
use std::sync::{Arc, Mutex};

// --- Fixtures ----------------------------------------------------------------

/// The gas of a transfer signed by `LocalWallet`: 21000 at 1 gwei.
const GAS: u128 = 21_000 * 1_000_000_000;

fn alice() -> LocalWallet {
    LocalWallet::new("0x0000000000000000000000000000000000000000000000000000000000000001")
}

fn bob() -> LocalWallet {
    LocalWallet::new("0x0000000000000000000000000000000000000000000000000000000000000002")
}

fn carol() -> LocalWallet {
    LocalWallet::new("0x0000000000000000000000000000000000000000000000000000000000000003")
}

/// SQLite in memory with the migrations of the rde. alice has 1000 RDEC; alice and bob passed KYC, carol did not.
/// The inbox gets every notification.
async fn ports_and_inbox() -> (Arc<Ports>, Arc<Mutex<Vec<Notification>>>) {
    let inbox = Arc::new(Mutex::new(vec![]));

    let database = SqliteDatabase::in_memory().await.unwrap();
    database.migrate(&sqlx::migrate!()).await.unwrap();

    let external_systems = ExternalSystems {
        blockchain: Arc::new(InMemoryBlockchain::new(vec![(
            alice().address().clone(),
            1000 * WEI_PER_RDEC,
        )])),
        kyc: Arc::new(InMemoryKycRegistry::new(vec![
            alice().address().clone(),
            bob().address().clone(),
        ])),
        notifier: Arc::new(InMemoryNotifier::new(Arc::clone(&inbox))),
    };

    (Arc::new(Ports::new(database, external_systems)), inbox)
}

async fn ports() -> Arc<Ports> {
    let (ports, _inbox) = ports_and_inbox().await;

    ports
}

/// What MetaMask does: asks the chain for the next nonce of `sender` and signs `rdec` RDEC to `recipient`.
async fn sign(
    ports: &Arc<Ports>,
    sender: &LocalWallet,
    recipient: &LocalWallet,
    rdec: u128,
) -> SignedTransaction {
    let next_nonce = ports.blockchain.next_nonce(sender.address()).await.unwrap();

    sender.sign_transfer(recipient.address(), rdec * WEI_PER_RDEC, next_nonce)
}

/// Runs the command in a transaction, with the commands of its policies stored in the outbox of the same one;
/// returns the policies fired. Those commands only run with `run_outbox`.
async fn run<Output>(
    ports: &Arc<Ports>,
    command: impl Command<Ports, Output = Output>,
) -> Result<Vec<&'static str>, Error> {
    let transaction = ports.begin().await?;

    let execution = command.execute(&transaction).await?;
    let fired_policies = transaction.outbox.send_events(execution.events).await?;

    transaction.commit().await?;

    Ok(fired_policies)
}

/// Runs every command waiting in the outbox, and the ones they fire in turn.
async fn run_outbox(ports: &Arc<Ports>) -> Vec<CommandRun> {
    let outbox_policy_processor = OutboxPolicyProcessor::new(Arc::clone(ports), command_registry());

    outbox_policy_processor.run_pending().await.unwrap()
}

/// Signs and creates a transfer, and returns its tx_hash.
async fn create_transfer(
    ports: &Arc<Ports>,
    sender: &LocalWallet,
    recipient: &LocalWallet,
    rdec: u128,
) -> TxHash {
    let create_transfer = CreateTransferCommand {
        signed_transaction: sign(ports, sender, recipient, rdec).await,
    };

    let transaction = ports.begin().await.unwrap();
    let create_transfer_execution = create_transfer.execute(&transaction).await.unwrap();
    transaction.commit().await.unwrap();

    create_transfer_execution.output
}

/// Accepts the transfer and runs the outbox, which chains it.
async fn accept_and_chain(ports: &Arc<Ports>, tx_hash: &TxHash) {
    run(ports, accept(tx_hash, &bob())).await.unwrap();
    run_outbox(ports).await;
}

fn done(command: &str) -> CommandRun {
    CommandRun {
        command: command.into(),
        error: None,
    }
}

fn create(signed_transaction: SignedTransaction) -> CreateTransferCommand {
    CreateTransferCommand { signed_transaction }
}

fn accept(tx_hash: &TxHash, recipient: &LocalWallet) -> AcceptTransferCommand {
    AcceptTransferCommand {
        tx_hash: tx_hash.clone(),
        recipient: recipient.address().clone(),
    }
}

fn violations(result: Result<Vec<&'static str>, Error>) -> Vec<&'static str> {
    match result {
        Err(Error::Domain(DomainError::Violations(v))) => v,
        other => panic!("expected a domain error, got {other:?}"),
    }
}

// --- Lane: creation ----------------------------------------------------------

#[tokio::test]
async fn new_transfer_is_pending_and_known_by_its_tx_hash() {
    let ports = ports().await;

    let tx_hash = create_transfer(&ports, &alice(), &bob(), 100).await;

    let transfer = ports.transfers.load(&tx_hash).await.unwrap();
    assert_eq!(transfer.status, TransferStatus::Pending);
    assert_eq!(&transfer.sender, alice().address());
    assert_eq!(transfer.amount, 100 * WEI_PER_RDEC);
    assert_eq!(transfer.nonce, 0);
}

#[tokio::test]
async fn new_transfer_notifies_the_recipient() {
    let (ports, inbox) = ports_and_inbox().await;

    let signed_transaction = sign(&ports, &alice(), &bob(), 100).await;
    let fired_policies = run(&ports, create(signed_transaction)).await.unwrap();

    assert_eq!(
        fired_policies,
        vec!["whenever a transfer is created, notify the recipient"]
    );
    assert!(
        inbox.lock().unwrap().is_empty(),
        "the command waits in the outbox"
    );

    assert_eq!(run_outbox(&ports).await, vec![done("notify_recipient")]);

    let inbox = inbox.lock().unwrap();
    assert_eq!(inbox.len(), 1);
    assert_eq!(&inbox[0].wallet, bob().address());
    assert!(
        inbox[0]
            .message
            .starts_with(&format!("{} sent you 100 RDEC.", alice().address()))
    );
}

#[tokio::test]
async fn rejected_creation_notifies_nobody() {
    let (ports, inbox) = ports_and_inbox().await;

    let signed_transaction = sign(&ports, &alice(), &carol(), 2000).await;
    let result = run(&ports, create(signed_transaction)).await;

    assert!(result.is_err());
    assert!(run_outbox(&ports).await.is_empty());
    assert!(inbox.lock().unwrap().is_empty());
}

#[tokio::test]
async fn creation_reports_every_broken_business_rule() {
    let ports = ports().await;

    let signed_transaction = sign(&ports, &alice(), &carol(), 2000).await;
    let result = run(&ports, create(signed_transaction)).await;

    assert_eq!(
        violations(result),
        vec!["sender has RDEC available", "recipient has KYC"]
    );
}

#[tokio::test]
async fn sender_without_kyc_cannot_create() {
    let ports = ports().await;

    let signed_transaction = sign(&ports, &carol(), &bob(), 1).await;
    let result = run(&ports, create(signed_transaction)).await;

    assert_eq!(
        violations(result),
        vec!["sender has RDEC available", "sender has KYC"]
    );
}

#[tokio::test]
async fn transfer_to_oneself_breaks_an_invariant() {
    let ports = ports().await;

    let signed_transaction = sign(&ports, &alice(), &alice(), 10).await;
    let result = run(&ports, create(signed_transaction)).await;

    assert_eq!(violations(result), vec!["sender is not the recipient"]);
}

#[tokio::test]
async fn the_nonce_must_be_the_next_one_of_the_sender() {
    let ports = ports().await;

    let signed_transaction = alice().sign_transfer(bob().address(), WEI_PER_RDEC, 5);
    let result = run(&ports, create(signed_transaction)).await;

    assert_eq!(
        violations(result),
        vec!["nonce is the next one of the sender"]
    );
}

#[tokio::test]
async fn a_sender_has_one_open_transfer_at_a_time() {
    let ports = ports().await;
    create_transfer(&ports, &alice(), &bob(), 100).await;

    // Same nonce: the chain has not taken the first transaction yet.
    let second = sign(&ports, &alice(), &bob(), 200).await;
    let result = run(&ports, create(second)).await;

    assert_eq!(violations(result), vec!["sender has no open transfer"]);
}

// --- Read model: pending transfers -------------------------------------------

#[tokio::test]
async fn recipient_sees_only_the_transfers_still_pending_for_them() {
    let ports = ports().await;
    let accepted_by_bob = create_transfer(&ports, &alice(), &bob(), 200).await;
    accept_and_chain(&ports, &accepted_by_bob).await;
    let to_bob = create_transfer(&ports, &alice(), &bob(), 100).await;

    let pending_transfers = PendingTransfersQuery {
        recipient: bob().address().clone(),
    }
    .execute(&ports)
    .await
    .unwrap();

    assert_eq!(
        pending_transfers.transfers,
        vec![PendingTransfer {
            tx_hash: to_bob,
            sender: alice().address().clone(),
            amount: (100 * WEI_PER_RDEC).to_string(),
        }]
    );
}

#[tokio::test]
async fn sender_has_no_pending_transfer_to_answer() {
    let ports = ports().await;
    create_transfer(&ports, &alice(), &bob(), 100).await;

    let pending_transfers = PendingTransfersQuery {
        recipient: alice().address().clone(),
    }
    .execute(&ports)
    .await
    .unwrap();

    assert!(pending_transfers.transfers.is_empty());
}

// --- Lane: response and chaining ---------------------------------------------

#[tokio::test]
async fn accepted_transfer_is_chained() {
    let ports = ports().await;
    let tx_hash = create_transfer(&ports, &alice(), &bob(), 100).await;

    let fired_policies = run(&ports, accept(&tx_hash, &bob())).await.unwrap();

    assert_eq!(
        fired_policies,
        vec!["whenever a transfer is accepted, chain it"]
    );
    assert_eq!(
        run_outbox(&ports).await,
        vec![done("chain_accepted_transfer")]
    );
    let transfer = ports.transfers.load(&tx_hash).await.unwrap();
    assert_eq!(transfer.status, TransferStatus::Accepted);
    assert!(transfer.chained);

    // 100 RDEC, 1% of decarbonization and the gas the transaction allows.
    let alice_pays = 100 * WEI_PER_RDEC + WEI_PER_RDEC + GAS;
    assert_eq!(
        ports
            .blockchain
            .available_rdec(alice().address())
            .await
            .unwrap(),
        1000 * WEI_PER_RDEC - alice_pays
    );
    assert_eq!(
        ports
            .blockchain
            .available_rdec(bob().address())
            .await
            .unwrap(),
        100 * WEI_PER_RDEC
    );

    let receipt = ports.blockchain.receipt(&tx_hash).await.unwrap().unwrap();
    assert!(receipt.succeeded);
    assert_eq!(receipt.block_number, 1);
}

#[tokio::test]
async fn only_the_recipient_accepts() {
    let ports = ports().await;
    let tx_hash = create_transfer(&ports, &alice(), &bob(), 100).await;

    let result = run(&ports, accept(&tx_hash, &alice())).await;

    assert_eq!(violations(result), vec!["only the recipient accepts"]);
}

#[tokio::test]
async fn rejected_transfer_is_chained_as_failed() {
    let ports = ports().await;
    let tx_hash = create_transfer(&ports, &alice(), &bob(), 100).await;

    let reject_transfer = RejectTransferCommand {
        tx_hash: tx_hash.clone(),
        recipient: bob().address().clone(),
    };
    let fired_policies = run(&ports, reject_transfer).await.unwrap();

    assert_eq!(
        fired_policies,
        vec!["whenever a transfer is rejected, chain it as failed"]
    );
    assert_eq!(
        run_outbox(&ports).await,
        vec![done("chain_failed_transfer")]
    );

    let transfer = ports.transfers.load(&tx_hash).await.unwrap();
    assert_eq!(transfer.status, TransferStatus::Rejected);
    assert!(transfer.chained);

    // In a block, as failed: no RDEC moved, and the nonce of alice moved on.
    let receipt = ports.blockchain.receipt(&tx_hash).await.unwrap().unwrap();
    assert!(!receipt.succeeded);
    assert_eq!(
        ports
            .blockchain
            .available_rdec(alice().address())
            .await
            .unwrap(),
        1000 * WEI_PER_RDEC
    );
    assert_eq!(
        ports
            .blockchain
            .next_nonce(alice().address())
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        violations(run(&ports, accept(&tx_hash, &bob())).await),
        vec!["transfer is still pending"]
    );
}

#[tokio::test]
async fn after_a_rejection_the_sender_sends_again() {
    let ports = ports().await;
    let rejected = create_transfer(&ports, &alice(), &bob(), 100).await;
    let reject_transfer = RejectTransferCommand {
        tx_hash: rejected,
        recipient: bob().address().clone(),
    };
    run(&ports, reject_transfer).await.unwrap();
    run_outbox(&ports).await;

    let tx_hash = create_transfer(&ports, &alice(), &bob(), 50).await;

    assert_eq!(ports.transfers.load(&tx_hash).await.unwrap().nonce, 1);
}

#[tokio::test]
async fn only_the_sender_cancels() {
    let ports = ports().await;
    let tx_hash = create_transfer(&ports, &alice(), &bob(), 100).await;

    let cancel_by_bob = CancelTransferCommand {
        tx_hash: tx_hash.clone(),
        sender: bob().address().clone(),
    };
    assert_eq!(
        violations(run(&ports, cancel_by_bob).await),
        vec!["only the sender cancels"]
    );

    let cancel_by_alice = CancelTransferCommand {
        tx_hash: tx_hash.clone(),
        sender: alice().address().clone(),
    };
    let fired_policies = run(&ports, cancel_by_alice).await.unwrap();

    assert_eq!(
        fired_policies,
        vec!["whenever a transfer is canceled, chain it as failed"]
    );
    assert_eq!(
        run_outbox(&ports).await,
        vec![done("chain_failed_transfer")]
    );
    assert_eq!(
        ports.transfers.load(&tx_hash).await.unwrap().status,
        TransferStatus::Canceled
    );
}

// --- Lane: cancellation in MetaMask ------------------------------------------

#[tokio::test]
async fn a_cancellation_signed_in_metamask_replaces_the_pending_transfer() {
    let ports = ports().await;
    let tx_hash = create_transfer(&ports, &alice(), &bob(), 100).await;

    let cancellation = alice().sign_cancellation(0);
    let send_cancellation = SendCancellationCommand {
        cancellation: cancellation.clone(),
    };

    let fired_policies = run(&ports, send_cancellation).await.unwrap();

    assert_eq!(
        fired_policies,
        vec!["whenever a transfer is replaced by a cancellation, chain the cancellation"]
    );
    assert_eq!(run_outbox(&ports).await, vec![done("chain_cancellation")]);

    let transfer = ports.transfers.load(&tx_hash).await.unwrap();
    let cancellation_receipt = ports
        .blockchain
        .receipt(cancellation.tx_hash())
        .await
        .unwrap()
        .unwrap();

    assert_eq!(transfer.status, TransferStatus::Canceled);
    assert_eq!(transfer.cancellation, Some(cancellation));
    assert!(cancellation_receipt.succeeded);
    assert_eq!(cancellation_receipt.gas_used, 0, "a cancellation is free");
    assert_eq!(
        ports.blockchain.receipt(&tx_hash).await.unwrap(),
        None,
        "the transfer never gets a block"
    );
    assert_eq!(
        ports
            .blockchain
            .next_nonce(alice().address())
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        ports
            .blockchain
            .available_rdec(alice().address())
            .await
            .unwrap(),
        1000 * WEI_PER_RDEC
    );
}

#[tokio::test]
async fn a_cancellation_needs_a_pending_transfer_with_its_nonce() {
    let ports = ports().await;

    let without_open_transfer = SendCancellationCommand {
        cancellation: alice().sign_cancellation(0),
    };

    assert!(matches!(
        run(&ports, without_open_transfer).await,
        Err(Error::Application(cerne::ApplicationError::NotFound(
            "open transfer"
        )))
    ));

    let tx_hash = create_transfer(&ports, &alice(), &bob(), 100).await;

    let with_another_nonce = SendCancellationCommand {
        cancellation: alice().sign_cancellation(7),
    };

    assert_eq!(
        violations(run(&ports, with_another_nonce).await),
        vec!["cancellation has the nonce of the transfer"]
    );

    let accept_transfer = AcceptTransferCommand {
        tx_hash,
        recipient: bob().address().clone(),
    };

    run(&ports, accept_transfer).await.unwrap();

    let after_the_recipient_accepted = SendCancellationCommand {
        cancellation: alice().sign_cancellation(0),
    };

    assert_eq!(
        violations(run(&ports, after_the_recipient_accepted).await),
        vec!["transfer is still pending"]
    );
}
