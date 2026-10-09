//! SQLite in a file, with a pool of connections: a transaction that reads and then writes must not fail because
//! another connection wrote in between. That is a server: a command reads an aggregate and saves it while
//! the outbox marks its own commands done.
#![cfg(feature = "sqlite")]

use cerne::sqlite::SqliteDatabase;
use std::time::Duration;

#[tokio::test]
async fn a_transaction_that_reads_then_writes_waits_for_the_other_writers() {
    let file = std::env::temp_dir().join(format!("cerne-transactions-{}.db", std::process::id()));
    let database = SqliteDatabase::connect(&format!("sqlite://{}?mode=rwc", file.display()), 5)
        .await
        .unwrap();

    database
        .execute(sqlx::query("CREATE TABLE counters (name TEXT PRIMARY KEY, value BIGINT NOT NULL)"))
        .await
        .unwrap();
    database
        .execute(sqlx::query("INSERT INTO counters VALUES ('transfers', 0), ('outbox', 0)"))
        .await
        .unwrap();

    // --- The command: begins, reads -------------------------------------------

    let transaction = database.begin().await.unwrap();

    transaction
        .fetch_one(sqlx::query("SELECT value FROM counters WHERE name = 'transfers'"))
        .await
        .unwrap();

    // --- The outbox, on another connection, writes in between ------------------

    let outbox_database = database.clone();
    let outbox_write = tokio::spawn(async move {
        outbox_database
            .execute(sqlx::query("UPDATE counters SET value = value + 1 WHERE name = 'outbox'"))
            .await
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    // --- The command writes and commits ----------------------------------------

    let command_write = transaction
        .execute(sqlx::query("UPDATE counters SET value = value + 1 WHERE name = 'transfers'"))
        .await;
    let command_commit = transaction.commit().await;

    let outbox_write = outbox_write.await.unwrap();

    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{}{suffix}", file.display()));
    }

    assert!(command_write.is_ok(), "{command_write:?}");
    assert!(command_commit.is_ok(), "{command_commit:?}");
    assert!(outbox_write.is_ok(), "{outbox_write:?}");
}
