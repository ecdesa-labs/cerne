use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};

/// What the e2e test writes into the generated project: the generated repository, on SQLite in memory.
const REPOSITORY_TEST: &str = r#"
use cerne::application::TransactionalPorts;
use cerne::domain::Entity;
use cerne::sqlite::SqliteDatabase;
use loja::domain::entities::order::{Order, OrderProps};
use loja::ports::Ports;

#[tokio::test]
async fn the_generated_repository_inserts_loads_and_updates() {
    let database = SqliteDatabase::in_memory().await.unwrap();
    database.migrate(&sqlx::migrate!()).await.unwrap();
    let ports = Ports::new(database);

    let transaction = ports.begin().await.unwrap();
    let order_id = transaction.orders.save(Order::new(OrderProps { qty: 3 }).unwrap()).await.unwrap();
    transaction.commit().await.unwrap();

    let order = ports.orders.load(&order_id).await.unwrap();
    ports.orders.save(Order { qty: 5, ..order }).await.unwrap();

    assert_eq!(ports.orders.load(&order_id).await.unwrap().qty, 5);
}
"#;

fn cerne(args: &[&str], dir: &Path) -> bool {
    Command::new(env!("CARGO_BIN_EXE_cerne"))
        .args(args)
        .current_dir(dir)
        .status()
        .unwrap()
        .success()
}

#[test]
fn generated_project_passes_clippy_without_touching_anything() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let tmp: PathBuf = env::temp_dir().join(format!("cerne-e2e-{}", std::process::id()));

    fs::create_dir_all(&tmp).unwrap();

    // --- cerne new -----------------------------------------------------------

    assert!(cerne(&["new", "loja", "--http", "jsonrpc"], &tmp));
    assert!(
        !cerne(&["new", "loja"], &tmp),
        "new must not overwrite a project"
    );

    let project = tmp.join("loja");

    // --- cerne g -------------------------------------------------------------

    assert!(cerne(
        &["g", "entity", "Order", "qty:i32", "id:u64", "--aggregate"],
        &project
    ));
    assert!(cerne(
        &["g", "entity", "OrderItem", "qty:i32", "sku:String"],
        &project
    ));
    assert!(cerne(&["g", "entity", "Tag"], &project));
    assert!(cerne(
        &[
            "g",
            "entity",
            "Payment",
            "payment_status=Pending:Pending,Paid",
            "method:Pix,Card",
            "--aggregate"
        ],
        &project
    ));
    assert!(cerne(
        &["g", "value_object", "Amount", "value:u64"],
        &project
    ));
    assert!(cerne(
        &[
            "g",
            "value_object",
            "Money",
            "amount:u64",
            "currency:String"
        ],
        &project
    ));
    assert!(cerne(
        &["g", "event", "OrderPlaced", "order_id:u64"],
        &project
    ));
    assert!(cerne(&["g", "event", "Pinged"], &project));
    assert!(cerne(
        &["g", "command", "PlaceOrder", "order_id:u64", "qty:i32"],
        &project
    ));
    assert!(cerne(
        &["g", "command", "ShipOrder", "order_id:u64", "--policy"],
        &project
    ));
    assert!(cerne(&["g", "port", "Notifier"], &project));
    assert!(cerne(
        &["g", "adapter", "SmtpNotifier", "Notifier"],
        &project
    ));
    assert!(!cerne(&["g", "adapter", "Smtp", "Mailer"], &project));
    assert!(
        !cerne(
            &["g", "endpoint", "PlaceOrder", "POST", "/orders"],
            &project
        ),
        "endpoint is for REST projects"
    );

    assert!(
        !cerne(&["g", "query", "OrderSummary", "order_id:u64"], &project),
        "a query needs its read model first"
    );
    assert!(cerne(
        &[
            "g",
            "read_model",
            "OrderSummary",
            "order_id:u64",
            "total:u64"
        ],
        &project
    ));
    assert!(cerne(
        &["g", "query", "OrderSummary", "order_id:u64"],
        &project
    ));

    assert!(
        !cerne(&["g", "entity", "Order"], &project),
        "g must not overwrite a post-it"
    );
    assert!(!cerne(&["g", "value_object", "Empty"], &project));
    assert!(!cerne(&["g", "event", "Bad", "qty"], &project));
    assert!(!cerne(
        &["g", "event", "Bad", "status:Pending,Paid"],
        &project
    ));
    assert!(!cerne(
        &["g", "entity", "Bad", "status:pending,Paid"],
        &project
    ));
    assert!(!cerne(
        &["g", "entity", "Bad", "status=Done:Pending,Paid"],
        &project
    ));
    assert!(!cerne(&["g", "entity", "Bad", "qty=1:i32"], &project));

    assert_eq!(
        fs::read_to_string(project.join("src/domain/value_objects/mod.rs")).unwrap(),
        "pub mod order_id;\npub mod order_item_id;\npub mod tag_id;\npub mod payment_id;\npub mod amount;\npub mod money;\n"
    );

    let payment = fs::read_to_string(project.join("src/domain/entities/payment.rs")).unwrap();

    assert!(payment.contains("pub enum PaymentPaymentStatus {"));
    assert!(payment.contains("payment_status: PaymentPaymentStatus::Pending,"));
    assert!(payment.contains("pub enum PaymentMethod {"));
    assert!(payment.contains("method: props.method,"));
    assert!(payment.contains("pub struct PaymentProps {\n    pub method: PaymentMethod,\n}"));

    let rpc = fs::read_to_string(project.join("src/infrastructure/http/rpc.rs")).unwrap();

    assert!(rpc.contains(
        r#""place_order" => methods.command::<PlaceOrderCommand>(request.params).await,"#
    ));
    assert!(
        !rpc.contains("ship_order"),
        "a policy command has no actor to call it"
    );
    assert!(!rpc.contains("match_single_binding"));

    let ports = fs::read_to_string(project.join("src/ports.rs")).unwrap();

    assert!(ports.contains("pub orders: Box<dyn Repository<Order>>,"));
    assert!(ports.contains("register::<ShipOrderCommand>()"));

    // --- cargo clippy and cargo test, with the generated repository on SQLite in memory

    fs::write(project.join("tests/repository.rs"), REPOSITORY_TEST).unwrap();

    let memory_and_jsonrpc_passed = cargo(
        &project,
        &workspace,
        &["clippy", "--all-targets", "--", "-D", "warnings"],
    ) && cargo(&project, &workspace, &["test"]);

    // --- Postgres and REST: cargo clippy -------------------------------------

    assert!(cerne(
        &["new", "vitrine", "--db", "postgres", "--http", "rest"],
        &tmp
    ));
    assert!(!cerne(&["new", "bad", "--db", "mysql"], &tmp));

    let vitrine = tmp.join("vitrine");

    assert!(cerne(
        &[
            "g",
            "entity",
            "Product",
            "name:String",
            "price:u64",
            "--aggregate"
        ],
        &vitrine
    ));
    assert!(cerne(
        &["g", "command", "AddProduct", "name:String", "price:u64"],
        &vitrine
    ));
    assert!(cerne(
        &["g", "endpoint", "AddProduct", "POST", "/products"],
        &vitrine
    ));
    assert!(cerne(
        &["g", "read_model", "Catalog", "total:u64"],
        &vitrine
    ));
    assert!(cerne(&["g", "query", "Catalog", "page:u32"], &vitrine));
    assert!(cerne(
        &["g", "endpoint", "Catalog", "GET", "/catalog"],
        &vitrine
    ));
    assert!(!cerne(
        &["g", "endpoint", "Missing", "POST", "/missing"],
        &vitrine
    ));

    let postgres_and_rest_passed = cargo(
        &vitrine,
        &workspace,
        &["clippy", "--all-targets", "--", "-D", "warnings"],
    );

    // --- SQLite file, no HTTP: cargo run runs the generated migrations ------

    assert!(cerne(&["new", "caixa", "--db", "sqlite"], &tmp));

    let caixa = tmp.join("caixa");

    assert!(cerne(
        &["g", "entity", "Sale", "total:u64", "--aggregate"],
        &caixa
    ));

    let sqlite_passed = cargo(
        &caixa,
        &workspace,
        &["clippy", "--all-targets", "--", "-D", "warnings"],
    ) && cargo(&caixa, &workspace, &["run"]);

    fs::remove_dir_all(&tmp).unwrap();

    assert!(memory_and_jsonrpc_passed);
    assert!(postgres_and_rest_passed);
    assert!(sqlite_passed);
}

/// Runs cargo in a generated project, with the repository's own crate standing in for the Git dependency.
fn cargo(project: &Path, workspace: &Path, args: &[&str]) -> bool {
    let manifest = project.join("Cargo.toml");
    let cerne_path = workspace.join("crates/cerne");
    let cargo_toml = fs::read_to_string(&manifest).unwrap().replace(
        r#"git = "https://github.com/ecdesa-labs/cerne""#,
        &format!("path = {:?}", cerne_path.display().to_string()),
    );

    fs::write(&manifest, cargo_toml).unwrap();
    fs::copy(workspace.join("Cargo.lock"), project.join("Cargo.lock")).unwrap();

    Command::new(env::var("CARGO").unwrap_or("cargo".into()))
        .args(args)
        .current_dir(project)
        .env("CARGO_TARGET_DIR", workspace.join("target/e2e"))
        .status()
        .unwrap()
        .success()
}
