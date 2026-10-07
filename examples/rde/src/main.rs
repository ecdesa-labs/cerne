use anyhow::Context;
use cerne::application::{Command, OutboxPolicyProcessor, Query, TransactionalPorts};
use cerne::sqlite::SqliteDatabase;
use rde::application::commands::accept_transfer::AcceptTransferCommand;
use rde::application::commands::create_transfer::CreateTransferCommand;
use rde::application::queries::pending_transfers::PendingTransfersQuery;
use rde::domain::services::units::{WEI_PER_RDEC, rdec};
use rde::infrastructure::in_memory_blockchain::InMemoryBlockchain;
use rde::infrastructure::in_memory_kyc_registry::InMemoryKycRegistry;
use rde::infrastructure::in_memory_notifier::InMemoryNotifier;
use rde::infrastructure::local_wallet::LocalWallet;
use rde::ports::{ExternalSystems, Ports, command_registry};
use std::sync::{Arc, Mutex};

/// Fixed keys, so every run has the same wallets. Never use them on a real chain.
const ALICE_PRIVATE_KEY: &str =
    "0x0000000000000000000000000000000000000000000000000000000000000001";
const BOB_PRIVATE_KEY: &str = "0x0000000000000000000000000000000000000000000000000000000000000002";

/// alice signs 100 RDEC to bob in her wallet, a policy notifies bob, bob finds the transfer among his pending ones
/// and accepts it, and another policy chains the transaction.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // --- Composition root: SQLite in memory + in-memory external systems -----

    let alice = LocalWallet::new(ALICE_PRIVATE_KEY);
    let bob = LocalWallet::new(BOB_PRIVATE_KEY);

    let database = SqliteDatabase::in_memory().await?;

    database.migrate(&sqlx::migrate!()).await?;

    let inbox = Arc::new(Mutex::new(vec![]));

    let external_systems = ExternalSystems {
        blockchain: Arc::new(InMemoryBlockchain::new(vec![(
            alice.address().clone(),
            1000 * WEI_PER_RDEC,
        )])),
        kyc: Arc::new(InMemoryKycRegistry::new(vec![
            alice.address().clone(),
            bob.address().clone(),
        ])),
        notifier: Arc::new(InMemoryNotifier::new(Arc::clone(&inbox))),
    };

    let ports = Arc::new(Ports::new(database, external_systems));

    let outbox_policy_processor =
        OutboxPolicyProcessor::new(Arc::clone(&ports), command_registry());

    // --- Sender: alice signs the transfer in her wallet and sends it ---------

    // MetaMask does this part: it asks the rde for the next nonce of alice, signs, and sends the signed transaction
    // in `eth_sendRawTransaction`. The rde never sees the private key.
    let next_nonce = ports.blockchain.next_nonce(alice.address()).await?;

    let signed_transaction = alice.sign_transfer(bob.address(), 100 * WEI_PER_RDEC, next_nonce);

    let create_transfer = CreateTransferCommand { signed_transaction };

    // The transfer and the commands of its policies are written in the same transaction: both or neither.
    let transaction = ports.begin().await?;

    // Every command returns an `Executed` with two parts:
    // - `output`: goes back to whoever sent the request (here, the tx_hash MetaMask follows);
    // - `events`: go to the outbox, which stores the commands of their policies.
    let create_transfer_execution = create_transfer.execute(&transaction).await?;

    let tx_hash = create_transfer_execution.output;

    let fired_policies = transaction
        .outbox
        .send_events(create_transfer_execution.events)
        .await?;

    transaction.commit().await?;

    println!("alice created the transfer {tx_hash}; policies fired: {fired_policies:?}");

    // The policy only stored its command: it runs now, each command in a transaction of its own.
    let command_runs = outbox_policy_processor.run_pending().await?;

    println!("the outbox ran {command_runs:?}");

    for notification in inbox.lock().unwrap().iter() {
        println!(
            "{} was notified: {}",
            notification.wallet, notification.message
        );
    }

    // --- Recipient: bob looks at his pending transfers -----------------------

    // A query only reads: no transaction, no events, no policies. Bob never gets alice's output, he finds the
    // transfer here.
    let pending_transfers_query = PendingTransfersQuery {
        recipient: bob.address().clone(),
    };

    let pending_transfers = pending_transfers_query.execute(&ports).await?;

    let transfer_from_alice = pending_transfers
        .transfers
        .iter()
        .find(|pending_transfer| &pending_transfer.sender == alice.address())
        .context("bob has no pending transfer from alice")?;

    println!(
        "bob has {} pending transfer(s); the one from alice is {}",
        pending_transfers.transfers.len(),
        rdec(transfer_from_alice.amount.parse()?)
    );

    // --- Recipient: bob accepts, and the policy chains the transaction -------

    let accept_transfer = AcceptTransferCommand {
        tx_hash: transfer_from_alice.tx_hash.clone(),
        recipient: bob.address().clone(),
    };

    let transaction = ports.begin().await?;

    let accept_transfer_execution = accept_transfer.execute(&transaction).await?;

    let fired_policies = transaction
        .outbox
        .send_events(accept_transfer_execution.events)
        .await?;

    transaction.commit().await?;

    println!("bob accepted it; policies fired: {fired_policies:?}");

    let command_runs = outbox_policy_processor.run_pending().await?;

    println!("the outbox ran {command_runs:?}");

    // --- Result --------------------------------------------------------------

    let transfer = ports.transfers.load(&tx_hash).await?;

    println!(
        "status: {:?}, chained: {}",
        transfer.status, transfer.chained
    );

    println!(
        "alice has {}, bob has {}",
        rdec(ports.blockchain.available_rdec(alice.address()).await?),
        rdec(ports.blockchain.available_rdec(bob.address()).await?)
    );

    Ok(())
}
