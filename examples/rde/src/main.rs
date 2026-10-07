use anyhow::Context;
use cerne::application::{Command, OutboxPolicyProcessor, Query, TransactionalPorts};
use cerne::sqlite::SqliteDatabase;
use rde::application::commands::accept_transfer::AcceptTransferCommand;
use rde::application::commands::create_transfer::CreateTransferCommand;
use rde::application::queries::pending_transfers::PendingTransfersQuery;
use rde::infrastructure::in_memory_blockchain::InMemoryBlockchain;
use rde::infrastructure::in_memory_kyc_registry::InMemoryKycRegistry;
use rde::infrastructure::in_memory_notifier::InMemoryNotifier;
use rde::ports::{ExternalSystems, Ports, command_registry};
use std::sync::{Arc, Mutex};

/// alice sends 100 RDEC to bob, a policy notifies bob, bob finds the transfer among his pending ones and accepts it,
/// and another policy chains the transaction.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // --- Composition root: SQLite in memory + in-memory external systems -----

    let database = SqliteDatabase::in_memory().await?;

    database.migrate(&sqlx::migrate!()).await?;

    let inbox = Arc::new(Mutex::new(vec![]));

    let external_systems = ExternalSystems {
        blockchain: Arc::new(InMemoryBlockchain::new(vec![("alice", 1000)])),
        kyc: Arc::new(InMemoryKycRegistry::new(vec!["alice", "bob"])),
        notifier: Arc::new(InMemoryNotifier::new(Arc::clone(&inbox))),
    };

    let ports = Arc::new(Ports::new(database, external_systems));

    let outbox_policy_processor =
        OutboxPolicyProcessor::new(Arc::clone(&ports), command_registry());

    // --- Sender: alice creates the transfer ----------------------------------

    let create_transfer = CreateTransferCommand {
        sender: "alice".into(),
        recipient: "bob".into(),
        amount: 100,
    };

    // The transfer and the commands of its policies are written in the same transaction: both or neither.
    let transaction = ports.begin().await?;

    // Every command returns an `Executed` with two parts:
    // - `output`: goes back to whoever sent the request (here, the tx_hash for alice);
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
        recipient: "bob".into(),
    };

    let pending_transfers = pending_transfers_query.execute(&ports).await?;

    let transfer_from_alice = pending_transfers
        .transfers
        .iter()
        .find(|pending_transfer| pending_transfer.sender == "alice")
        .context("bob has no pending transfer from alice")?;

    println!(
        "bob has {} pending transfer(s); the one from alice is {} RDEC",
        pending_transfers.transfers.len(),
        transfer_from_alice.amount
    );

    // --- Recipient: bob accepts, and the policy chains the transaction -------

    let accept_transfer = AcceptTransferCommand {
        tx_hash: transfer_from_alice.tx_hash.clone(),
        recipient: "bob".into(),
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
        "alice has {} RDEC, bob has {} RDEC",
        ports.blockchain.available_rdec("alice").await?,
        ports.blockchain.available_rdec("bob").await?
    );

    Ok(())
}
