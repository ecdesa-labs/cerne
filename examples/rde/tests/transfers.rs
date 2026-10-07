use cerne::application::{Command, CommandRun, OutboxPolicyProcessor, Query, TransactionalPorts};
use cerne::sqlite::SqliteDatabase;
use cerne::{DomainError, Error};
use rde::application::commands::accept_transfer::AcceptTransferCommand;
use rde::application::commands::cancel_transfer::CancelTransferCommand;
use rde::application::commands::create_transfer::CreateTransferCommand;
use rde::application::commands::reject_transfer::RejectTransferCommand;
use rde::application::queries::pending_transfers::PendingTransfersQuery;
use rde::application::read_models::pending_transfers::PendingTransfer;
use rde::domain::entities::transfer::TransferStatus;
use rde::domain::value_objects::tx_hash::TxHash;
use rde::infrastructure::in_memory_blockchain::InMemoryBlockchain;
use rde::infrastructure::in_memory_kyc_registry::InMemoryKycRegistry;
use rde::infrastructure::in_memory_notifier::{InMemoryNotifier, Notification};
use rde::ports::{ExternalSystems, Ports, command_registry};
use std::sync::{Arc, Mutex};

// --- Fixtures ----------------------------------------------------------------

/// SQLite in memory with the migrations of the rde. alice has 1000 RDEC; alice and bob passed KYC, carol did not.
/// The inbox gets every notification.
async fn ports_and_inbox() -> (Arc<Ports>, Arc<Mutex<Vec<Notification>>>) {
    let inbox = Arc::new(Mutex::new(vec![]));

    let database = SqliteDatabase::in_memory().await.unwrap();
    database.migrate(&sqlx::migrate!()).await.unwrap();

    let external_systems = ExternalSystems {
        blockchain: Arc::new(InMemoryBlockchain::new(vec![("alice", 1000)])),
        kyc: Arc::new(InMemoryKycRegistry::new(vec!["alice", "bob"])),
        notifier: Arc::new(InMemoryNotifier::new(Arc::clone(&inbox))),
    };

    (Arc::new(Ports::new(database, external_systems)), inbox)
}

async fn ports() -> Arc<Ports> {
    let (ports, _inbox) = ports_and_inbox().await;

    ports
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

/// Creates a transfer and returns its tx_hash.
async fn create_transfer(ports: &Arc<Ports>, sender: &str, recipient: &str, amount: u64) -> TxHash {
    let create_transfer = CreateTransferCommand {
        sender: sender.into(),
        recipient: recipient.into(),
        amount,
    };

    let transaction = ports.begin().await.unwrap();
    let create_transfer_execution = create_transfer.execute(&transaction).await.unwrap();
    transaction.commit().await.unwrap();

    create_transfer_execution.output
}

fn done(command: &str) -> CommandRun {
    CommandRun {
        command: command.into(),
        error: None,
    }
}

fn create(sender: &str, recipient: &str, amount: u64) -> CreateTransferCommand {
    CreateTransferCommand {
        sender: sender.into(),
        recipient: recipient.into(),
        amount,
    }
}

fn accept(tx_hash: &TxHash, recipient: &str) -> AcceptTransferCommand {
    AcceptTransferCommand {
        tx_hash: tx_hash.clone(),
        recipient: recipient.into(),
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

    let tx_hash = create_transfer(&ports, "alice", "bob", 100).await;

    let transfer = ports.transfers.load(&tx_hash).await.unwrap();
    assert_eq!(transfer.status, TransferStatus::Pending);
    assert_eq!(transfer.nonce, 0);
}

#[tokio::test]
async fn new_transfer_notifies_the_recipient() {
    let (ports, inbox) = ports_and_inbox().await;

    let fired_policies = run(&ports, create("alice", "bob", 100)).await.unwrap();

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
    assert_eq!(inbox[0].wallet, "bob");
    assert!(inbox[0].message.starts_with("alice sent you 100 RDEC."));
}

#[tokio::test]
async fn rejected_creation_notifies_nobody() {
    let (ports, inbox) = ports_and_inbox().await;

    let result = run(&ports, create("alice", "carol", 2000)).await;

    assert!(result.is_err());
    assert!(run_outbox(&ports).await.is_empty());
    assert!(inbox.lock().unwrap().is_empty());
}

#[tokio::test]
async fn creation_reports_every_broken_business_rule() {
    let ports = ports().await;

    let result = run(&ports, create("alice", "carol", 2000)).await;

    assert_eq!(
        violations(result),
        vec!["sender has RDEC available", "recipient has KYC"]
    );
}

#[tokio::test]
async fn sender_without_kyc_cannot_create() {
    let ports = ports().await;

    let result = run(&ports, create("carol", "bob", 1)).await;

    assert_eq!(
        violations(result),
        vec!["sender has RDEC available", "sender has KYC"]
    );
}

#[tokio::test]
async fn transfer_to_oneself_breaks_an_invariant() {
    let ports = ports().await;

    let result = run(&ports, create("alice", "alice", 10)).await;

    assert_eq!(violations(result), vec!["sender is not the recipient"]);
}

// --- Read model: pending transfers -------------------------------------------

#[tokio::test]
async fn recipient_sees_only_the_transfers_still_pending_for_them() {
    let ports = ports().await;
    let to_bob = create_transfer(&ports, "alice", "bob", 100).await;
    let accepted_by_bob = create_transfer(&ports, "alice", "bob", 200).await;
    run(&ports, accept(&accepted_by_bob, "bob")).await.unwrap();

    let pending_transfers = PendingTransfersQuery {
        recipient: "bob".into(),
    }
    .execute(&ports)
    .await
    .unwrap();

    assert_eq!(
        pending_transfers.transfers,
        vec![PendingTransfer {
            tx_hash: to_bob,
            sender: "alice".into(),
            amount: 100,
        }]
    );
}

#[tokio::test]
async fn sender_has_no_pending_transfer_to_answer() {
    let ports = ports().await;
    create_transfer(&ports, "alice", "bob", 100).await;

    let pending_transfers = PendingTransfersQuery {
        recipient: "alice".into(),
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
    let tx_hash = create_transfer(&ports, "alice", "bob", 100).await;

    let fired_policies = run(&ports, accept(&tx_hash, "bob")).await.unwrap();

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
    assert_eq!(ports.blockchain.available_rdec("alice").await.unwrap(), 898); // 100 + 1% + 1 of gas
    assert_eq!(ports.blockchain.available_rdec("bob").await.unwrap(), 100);
}

#[tokio::test]
async fn only_the_recipient_accepts() {
    let ports = ports().await;
    let tx_hash = create_transfer(&ports, "alice", "bob", 100).await;

    let result = run(&ports, accept(&tx_hash, "alice")).await;

    assert_eq!(violations(result), vec!["only the recipient accepts"]);
}

#[tokio::test]
async fn rejected_transfer_is_never_chained() {
    let ports = ports().await;
    let tx_hash = create_transfer(&ports, "alice", "bob", 100).await;

    let reject_transfer = RejectTransferCommand {
        tx_hash: tx_hash.clone(),
        recipient: "bob".into(),
    };
    let fired_policies = run(&ports, reject_transfer).await.unwrap();

    assert!(fired_policies.is_empty());
    assert_eq!(
        ports.transfers.load(&tx_hash).await.unwrap().status,
        TransferStatus::Rejected
    );
    assert_eq!(
        violations(run(&ports, accept(&tx_hash, "bob")).await),
        vec!["transfer is still pending"]
    );
}

#[tokio::test]
async fn only_the_sender_cancels() {
    let ports = ports().await;
    let tx_hash = create_transfer(&ports, "alice", "bob", 100).await;

    let cancel_by_bob = CancelTransferCommand {
        tx_hash: tx_hash.clone(),
        sender: "bob".into(),
    };
    assert_eq!(
        violations(run(&ports, cancel_by_bob).await),
        vec!["only the sender cancels"]
    );

    let cancel_by_alice = CancelTransferCommand {
        tx_hash: tx_hash.clone(),
        sender: "alice".into(),
    };
    run(&ports, cancel_by_alice).await.unwrap();

    assert_eq!(
        ports.transfers.load(&tx_hash).await.unwrap().status,
        TransferStatus::Canceled
    );
}

// --- Hotspots ----------------------------------------------------------------

/// Hotspot 1 ("A criação reserva o RDEC?"), as the board stands today: creating does not reserve.
/// Two transfers that together exceed the balance are both created.
#[tokio::test]
async fn hotspot_1_creating_does_not_reserve_rdec() {
    let ports = ports().await;

    let first = create_transfer(&ports, "alice", "bob", 600).await;
    let second = create_transfer(&ports, "alice", "bob", 500).await;

    assert_eq!(
        ports.transfers.load(&first).await.unwrap().status,
        TransferStatus::Pending
    );
    assert_eq!(
        ports.transfers.load(&second).await.unwrap().status,
        TransferStatus::Pending
    );
}

/// Hotspot 5: the nonce goes into the tx_hash at creation, but only advances on the chain after acceptance.
/// Two pending transfers of a sender carry the same nonce, so the chain refuses the second one accepted.
#[tokio::test]
async fn hotspot_5_pending_transfers_of_a_sender_share_a_nonce() {
    let ports = ports().await;
    let first = create_transfer(&ports, "alice", "bob", 100).await;
    let second = create_transfer(&ports, "alice", "bob", 200).await;

    run(&ports, accept(&first, "bob")).await.unwrap();
    run(&ports, accept(&second, "bob")).await.unwrap();

    assert_eq!(
        run_outbox(&ports).await,
        vec![
            done("chain_accepted_transfer"),
            CommandRun {
                command: "chain_accepted_transfer".into(),
                error: Some("chain refused: alice expected nonce 1, got 0".into()),
            }
        ]
    );
    let second = ports.transfers.load(&second).await.unwrap();
    assert_eq!(second.status, TransferStatus::Accepted);
    assert!(!second.chained);
}
