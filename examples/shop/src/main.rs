use cerne::application::OutboxPolicyProcessor;
use cerne::sqlite::SqliteDatabase;
use shop::composition_root::{CompositionRoot, CompositionRootConstructor, command_registry};
use shop::infrastructure::http::router;
use shop::infrastructure::in_memory_catalog::InMemoryCatalog;
use shop::infrastructure::in_memory_payments::InMemoryPayments;
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // --- Composition root ----------------------------------------------------

    let database_url = std::env::var("DATABASE_URL").unwrap_or("sqlite://shop.db?mode=rwc".into());

    let database = SqliteDatabase::connect(&database_url, 5).await?;

    database.migrate(&sqlx::migrate!()).await?;

    let catalog = Arc::new(InMemoryCatalog {
        products: vec![("mug", 3000, 10), ("t-shirt", 5000, 3)],
    });
    let payments = Arc::new(InMemoryPayments::default());

    let composition_root_constructor = CompositionRootConstructor {
        database,
        catalog,
        payments,
    };

    let composition_root = Arc::new(CompositionRoot::new(composition_root_constructor));

    // --- Outbox: the commands of the policies --------------------------------

    let outbox_policy_processor = OutboxPolicyProcessor::new(Arc::clone(&composition_root), command_registry());

    tokio::spawn(async move {
        outbox_policy_processor
            .run_every(Duration::from_millis(200), |error| eprintln!("outbox: {error}"))
            .await
    });

    // --- HTTP: REST ----------------------------------------------------------

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;

    println!("listening on http://127.0.0.1:3000");

    axum::serve(listener, router(composition_root)).await?;

    Ok(())
}
