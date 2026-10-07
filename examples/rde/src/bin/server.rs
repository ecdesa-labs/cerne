//! The rde as a JSON-RPC server, the node MetaMask calls: the transfers in `rde.db`, the chain in `rde-chain.db`, and
//! the other external systems (KYC, notifier) in memory.
//!
//! - Every start begins a new chain: the server deletes both files first, so the transfers and the chain always
//!   match. MetaMask remembers the nonces of the old chain: in MetaMask, Settings → Advanced → "Clear activity tab
//!   data" after restarting the server.
//! - `RDE_WALLETS` lists the wallets that start with 1000 RDEC and pass KYC, separated by commas
//!   (`RDE_WALLETS=0x6d13…,0x9100…`). Without it, no wallet has RDEC and no transfer passes the business rules.
//!
//! In MetaMask, add a network with the RPC URL `http://127.0.0.1:3000/rpc`, chain id 8808 and currency RDEC.

use cerne::application::OutboxPolicyProcessor;
use cerne::domain::ValueObject;
use cerne::sqlite::SqliteDatabase;
use rde::domain::services::units::WEI_PER_RDEC;
use rde::domain::value_objects::address::Address;
use rde::infrastructure::http::router;
use rde::infrastructure::in_memory_kyc_registry::InMemoryKycRegistry;
use rde::infrastructure::in_memory_notifier::InMemoryNotifier;
use rde::infrastructure::sqlite_blockchain::SqliteBlockchain;
use rde::ports::{ExternalSystems, Ports, command_registry};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // --- Composition root: two SQLite files, started over ---------------------

    for file in ["rde.db", "rde-chain.db"] {
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{file}{suffix}"));
        }
    }

    let database = SqliteDatabase::connect("sqlite://rde.db?mode=rwc", 5).await?;

    database.migrate(&sqlx::migrate!()).await?;

    let chain_database = SqliteDatabase::connect("sqlite://rde-chain.db?mode=rwc", 5).await?;

    let wallets = std::env::var("RDE_WALLETS")
        .unwrap_or_default()
        .split(',')
        .filter(|wallet| !wallet.trim().is_empty())
        .map(|wallet| Address::new(wallet.trim().to_string()))
        .collect::<Result<Vec<_>, _>>()?;

    let balances = wallets
        .iter()
        .map(|wallet| (wallet.clone(), 1000 * WEI_PER_RDEC))
        .collect();

    let external_systems = ExternalSystems {
        blockchain: Arc::new(SqliteBlockchain::open(chain_database, balances).await?),
        kyc: Arc::new(InMemoryKycRegistry::new(wallets.clone())),
        notifier: Arc::new(InMemoryNotifier::new(Arc::new(Mutex::new(vec![])))),
    };

    let ports = Arc::new(Ports::new(database, external_systems));

    // --- Outbox: the commands of the policies run in the background ----------

    let outbox_policy_processor =
        OutboxPolicyProcessor::new(Arc::clone(&ports), command_registry());

    tokio::spawn(async move {
        outbox_policy_processor
            .run_every(Duration::from_millis(200), |error| {
                eprintln!("outbox: {error}")
            })
            .await
    });

    // --- HTTP: JSON-RPC on POST /rpc -----------------------------------------

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;

    println!("rde listening on http://127.0.0.1:3000/rpc (chain id 8808)");

    for wallet in &wallets {
        println!("{wallet}: 1000 RDEC, KYC verified");
    }

    axum::serve(listener, router(ports)).await?;

    Ok(())
}
