use cerne::application::{Command, InlinePolicyProcessor, PolicyProcessor};
use rde::application::commands::accept_transfer::AcceptTransferCommand;
use rde::application::commands::create_transfer::CreateTransferCommand;
use rde::infrastructure::in_memory_blockchain::InMemoryBlockchain;
use rde::infrastructure::in_memory_kyc_registry::InMemoryKycRegistry;
use rde::infrastructure::in_memory_notifier::InMemoryNotifier;
use rde::infrastructure::in_memory_repository::InMemoryRepository;
use rde::ports::Ports;
use std::sync::{Arc, Mutex};

/// alice sends 100 RDEC to bob, a policy notifies bob, bob accepts, and another policy chains the transaction.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // --- Composition root: in-memory adapters --------------------------------

    let inbox = Arc::new(Mutex::new(vec![]));

    let ports = Arc::new(Ports {
        transfers: Box::new(InMemoryRepository::new(vec![])),
        blockchain: Box::new(InMemoryBlockchain::new(vec![("alice", 1000)])),
        kyc: Box::new(InMemoryKycRegistry::new(vec!["alice", "bob"])),
        notifier: Box::new(InMemoryNotifier::new(Arc::clone(&inbox))),
    });

    let sync_policy_processor = InlinePolicyProcessor::new(Arc::clone(&ports));

    // --- Sender: alice creates the transfer ----------------------------------

    let create_transfer = CreateTransferCommand {
        sender: "alice".into(),
        recipient: "bob".into(),
        amount: 100,
    };

    // Every command returns an `Executed` with two parts:
    // - `output`: goes back to whoever sent the request (here, the tx_hash for alice);
    // - `events`: go to the policy processor, which triggers their policies.
    let create_transfer_execution = create_transfer.execute(&ports).await?;

    let tx_hash = create_transfer_execution.output;

    let fired_policies = sync_policy_processor
        .send_events(create_transfer_execution.events)
        .await?;

    println!("alice created the transfer {tx_hash}; policies fired: {fired_policies:?}");

    // The policy already ran: bob got his notification before `send_events` returned.
    for notification in inbox.lock().unwrap().iter() {
        println!(
            "{} was notified: {}",
            notification.wallet, notification.message
        );
    }

    // --- Recipient: bob accepts, and the policy chains the transaction -------

    let accept_transfer = AcceptTransferCommand {
        tx_hash: tx_hash.clone(),
        recipient: "bob".into(),
    };

    let accept_transfer_execution = accept_transfer.execute(&ports).await?;

    let fired_policies = sync_policy_processor
        .send_events(accept_transfer_execution.events)
        .await?;

    println!("bob accepted it; policies fired: {fired_policies:?}");

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
