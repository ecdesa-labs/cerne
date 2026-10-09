use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};

/// What the e2e test writes into the generated project: the generated repository, on SQLite in memory.
const REPOSITORY_TEST: &str = r#"
use cerne::application::TransactionalPorts;
use cerne::domain::Entity;
use cerne::sqlite::SqliteDatabase;
use loja::domain::entities::order::{Order, OrderConstructor};
use loja::ports::Ports;

#[tokio::test]
async fn the_generated_repository_inserts_loads_and_updates() {
    let database = SqliteDatabase::in_memory().await.unwrap();
    database.migrate(&sqlx::migrate!()).await.unwrap();
    let ports = Ports::new(database);

    let transaction = ports.begin().await.unwrap();
    let order_id = transaction.orders.save(Order::new(OrderConstructor { qty: 3 }).unwrap()).await.unwrap();
    transaction.commit().await.unwrap();

    let order = ports.orders.load(&order_id).await.unwrap();
    ports.orders.save(Order { qty: 5, ..order }).await.unwrap();

    assert_eq!(ports.orders.load(&order_id).await.unwrap().qty, 5);
}
"#;

/// What the e2e test writes into the Postgres project: the generated repository, on a database of its own created in
/// the Postgres of `DATABASE_URL` (the CI starts one).
const POSTGRES_REPOSITORY_TEST: &str = r#"
use cerne::application::TransactionalPorts;
use cerne::domain::Entity;
use cerne::postgres::PostgresDatabase;
use vitrine::domain::entities::product::{Product, ProductKind, ProductConstructor};
use vitrine::ports::Ports;

#[tokio::test]
async fn the_generated_repository_inserts_loads_and_updates_on_postgres() {
    let database_url = std::env::var("DATABASE_URL").unwrap();
    let server = PostgresDatabase::connect(&database_url, 1).await.unwrap();
    let database_name = format!("cerne_e2e_{}", std::process::id());

    server.execute(sqlx::query(&format!("DROP DATABASE IF EXISTS {database_name}"))).await.unwrap();
    server.execute(sqlx::query(&format!("CREATE DATABASE {database_name}"))).await.unwrap();

    let (server_url, _) = database_url.rsplit_once('/').unwrap();
    let database = PostgresDatabase::connect(&format!("{server_url}/{database_name}"), 2).await.unwrap();
    database.migrate(&sqlx::migrate!()).await.unwrap();
    let ports = Ports::new(database);

    let product = Product::new(ProductConstructor {
        name: "Mug".into(),
        price: 30,
        kind: ProductKind::Physical,
        weight: 0.4,
        available: true,
    })
    .unwrap();

    let transaction = ports.begin().await.unwrap();
    let product_id = transaction.products.save(product).await.unwrap();
    transaction.commit().await.unwrap();

    let product = ports.products.load(&product_id).await.unwrap();
    ports.products.save(Product { price: 35, kind: ProductKind::Digital, available: false, ..product }).await.unwrap();

    let product = ports.products.load(&product_id).await.unwrap();

    assert_eq!((product.name.as_str(), product.price, product.weight), ("Mug", 35, 0.4));
    assert!(matches!(product.kind, ProductKind::Digital));
    assert!(!product.available);

    drop(ports);
    server.execute(sqlx::query(&format!("DROP DATABASE {database_name} WITH (FORCE)"))).await.unwrap();
}
"#;

/// What the e2e test writes into the REST project: a `POST` and a `GET` straight to the generated router.
/// What the e2e test writes into the project that got its database from `cerne g db`: the repository of the
/// aggregate that existed before the database.
const NUCLEO_REPOSITORY_TEST: &str = r#"
use cerne::application::TransactionalPorts;
use cerne::domain::Entity;
use cerne::sqlite::SqliteDatabase;
use nucleo::domain::entities::order::{Order, OrderConstructor, OrderStatus};
use nucleo::ports::Ports;

#[tokio::test]
async fn the_aggregate_from_before_the_database_gets_a_repository() {
    let database = SqliteDatabase::in_memory().await.unwrap();
    database.migrate(&sqlx::migrate!()).await.unwrap();
    let ports = Ports::new(database);

    let order = Order::new(OrderConstructor { product: "mug".into(), quantity: 2 }).unwrap();

    let transaction = ports.begin().await.unwrap();
    let order_id = transaction.orders.save(order).await.unwrap();
    transaction.commit().await.unwrap();

    let order = ports.orders.load(&order_id).await.unwrap();

    assert_eq!((order.product.as_str(), order.quantity, order.status), ("mug", 2, OrderStatus::Placed));
}
"#;

const REST_TEST: &str = r##"
use axum::body::Body;
use axum::http::Request;
use caixa::infrastructure::http::router;
use caixa::ports::Ports;
use cerne::sqlite::SqliteDatabase;
use std::sync::Arc;
use tower::ServiceExt;

async fn send(request: Request<Body>) -> (u16, String) {
    let database = SqliteDatabase::in_memory().await.unwrap();
    database.migrate(&sqlx::migrate!()).await.unwrap();
    let router = router(Arc::new(Ports::new(database)));

    let response = router.oneshot(request).await.unwrap();
    let status = response.status().as_u16();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();

    (status, String::from_utf8(bytes.to_vec()).unwrap())
}

fn post(path: &str, body: &str) -> Request<Body> {
    Request::post(path)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

#[tokio::test]
async fn the_generated_post_executes_the_command() {
    assert_eq!(send(post("/sales", r#"{"total": 30}"#)).await, (200, "null".into()));
    assert_eq!(send(post("/sales", "{}")).await.0, 422, "the body is not a RegisterSaleCommand");
}

#[tokio::test]
async fn the_generated_get_executes_the_query() {
    let get = |uri: &str| Request::get(uri).body(Body::empty()).unwrap();

    assert_eq!(send(get("/till?total=30")).await, (200, r#"{"total":30}"#.into()));
    assert_eq!(send(get("/till?total=abc")).await.0, 400, "the query string is not a TillQuery");
}
"##;

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

    assert!(cerne(&["new", "loja", "--db", "memory", "--http", "jsonrpc"], &tmp));
    assert!(!cerne(&["new", "loja"], &tmp), "new must not overwrite a project");

    let project = tmp.join("loja");

    assert!(!cerne(&["g", "http", "rest"], &project), "the project already speaks jsonrpc");

    // --- cerne g -------------------------------------------------------------

    assert!(cerne(&["g", "entity", "Order", "qty:i32", "id:u64", "--aggregate"], &project));
    assert!(cerne(&["g", "entity", "OrderItem", "qty:i32", "sku:String"], &project));
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
    assert!(cerne(&["g", "value_object", "Amount", "value:u64"], &project));
    assert!(cerne(&["g", "value_object", "Money", "amount:u64", "currency:String"], &project));
    assert!(cerne(&["g", "event", "OrderPlaced", "order_id:u64"], &project));
    assert!(cerne(&["g", "event", "Pinged"], &project));
    assert!(cerne(&["g", "command", "PlaceOrder", "order_id:u64", "qty:i32"], &project));
    assert!(cerne(&["g", "command", "ShipOrder", "order_id:u64", "--policy"], &project));
    assert!(cerne(&["g", "port", "Notifier"], &project));
    assert!(cerne(&["g", "adapter", "SmtpNotifier", "Notifier"], &project));
    assert!(!cerne(&["g", "adapter", "Smtp", "Mailer"], &project));
    assert!(!cerne(&["g", "endpoint", "PlaceOrder", "POST", "/orders"], &project), "endpoint is for REST projects");

    assert!(!cerne(&["g", "query", "OrderSummary", "order_id:u64"], &project), "a query needs its read model first");
    assert!(cerne(&["g", "read_model", "OrderSummary", "order_id:u64", "total:u64"], &project));
    assert!(cerne(&["g", "query", "OrderSummary", "order_id:u64"], &project));

    assert!(!cerne(&["g", "entity", "Order"], &project), "g must not overwrite a post-it");
    assert!(!cerne(&["g", "value_object", "Empty"], &project));
    assert!(!cerne(&["g", "event", "Bad", "qty"], &project));
    assert!(!cerne(&["g", "event", "Bad", "status:Pending,Paid"], &project));
    assert!(!cerne(&["g", "entity", "Bad", "status:pending,Paid"], &project));
    assert!(!cerne(&["g", "entity", "Bad", "status=Done:Pending,Paid"], &project));
    assert!(!cerne(&["g", "entity", "Bad", "qty=1:i32"], &project));

    assert_eq!(
        fs::read_to_string(project.join("src/domain/value_objects/mod.rs")).unwrap(),
        "pub mod order_id;\npub mod order_item_id;\npub mod tag_id;\npub mod payment_id;\npub mod amount;\npub mod money;\n"
    );

    let payment = fs::read_to_string(project.join("src/domain/entities/payment.rs")).unwrap();

    assert!(payment.contains("#[derive(Debug, Clone, Copy, Default, PartialEq)]\npub enum PaymentPaymentStatus {\n    #[default]\n    Pending,"));
    assert!(payment.contains("#[derive(Debug, Clone, Copy, PartialEq)]\npub enum PaymentMethod {"));
    assert!(payment.contains("#[aggregate]\n#[derive(Debug, Clone, PartialEq)]\npub struct Payment {"));
    assert!(payment.contains(
        "    #[skip_constructor]\n    pub payment_status: PaymentPaymentStatus,\n    pub method: PaymentMethod,"
    ));

    let rpc = fs::read_to_string(project.join("src/infrastructure/http/rpc.rs")).unwrap();

    assert!(rpc.contains(r#""place_order" => methods.command::<PlaceOrderCommand>(request.params).await,"#));
    assert!(!rpc.contains("ship_order"), "a policy command has no actor to call it");
    assert!(!rpc.contains("match_single_binding"));

    let ports = fs::read_to_string(project.join("src/ports.rs")).unwrap();

    assert!(ports.contains("pub orders: Box<dyn Repository<Order>>,"));
    assert!(ports.contains("register::<ShipOrderCommand>()"));

    // --- cargo clippy and cargo test, with the generated repository on SQLite in memory

    fs::write(project.join("tests/repository.rs"), REPOSITORY_TEST).unwrap();

    let memory_and_jsonrpc_passed = cargo(&project, &workspace, &["clippy", "--all-targets", "--", "-D", "warnings"])
        && cargo(&project, &workspace, &["test"]);

    // --- Postgres and REST: cargo clippy, and cargo test with DATABASE_URL ---

    assert!(cerne(&["new", "vitrine", "--db", "postgres", "--http", "rest"], &tmp));
    assert!(!cerne(&["new", "bad", "--db", "mysql"], &tmp));

    let vitrine = tmp.join("vitrine");

    assert!(cerne(
        &[
            "g",
            "entity",
            "Product",
            "name:String",
            "price:u64",
            "kind:Physical,Digital",
            "weight:f64",
            "available:bool",
            "--aggregate"
        ],
        &vitrine
    ));
    assert!(cerne(&["g", "command", "AddProduct", "name:String", "price:u64"], &vitrine));
    assert!(cerne(&["g", "endpoint", "AddProduct", "POST", "/products"], &vitrine));
    assert!(cerne(&["g", "read_model", "Catalog", "total:u64"], &vitrine));
    assert!(cerne(&["g", "query", "Catalog", "page:u32"], &vitrine));
    assert!(cerne(&["g", "endpoint", "Catalog", "GET", "/catalog"], &vitrine));
    assert!(!cerne(&["g", "endpoint", "Missing", "POST", "/missing"], &vitrine));

    fs::write(vitrine.join("tests/repository.rs"), POSTGRES_REPOSITORY_TEST).unwrap();

    let postgres_and_rest_passed = cargo(&vitrine, &workspace, &["clippy", "--all-targets", "--", "-D", "warnings"])
        && match env::var("DATABASE_URL") {
            // Without a Postgres (the CI job postgres starts one), the repository only goes through clippy.
            Err(_) => true,
            Ok(database_url) => cargo_on_postgres(&vitrine, &workspace, &["test"], &database_url),
        };

    // --- SQLite file, no HTTP: cargo run runs the generated migrations ------

    assert!(cerne(&["new", "caixa", "--db", "sqlite"], &tmp));

    let caixa = tmp.join("caixa");

    assert!(cerne(&["g", "entity", "Sale", "total:u64", "--aggregate"], &caixa));
    assert!(cerne(&["g", "command", "RegisterSale", "total:u64"], &caixa));
    assert!(cerne(&["g", "read_model", "Till", "total:u64"], &caixa));
    assert!(cerne(&["g", "query", "Till", "total:u64"], &caixa));

    let sqlite_passed = cargo(&caixa, &workspace, &["clippy", "--all-targets", "--", "-D", "warnings"])
        && cargo(&caixa, &workspace, &["run"]);

    // --- cerne g http rest: the same project, now with REST -----------------

    assert!(!cerne(&["g", "http", "soap"], &caixa));
    assert!(cerne(&["g", "http", "rest"], &caixa));
    assert!(!cerne(&["g", "http", "rest"], &caixa), "the project already speaks rest");
    assert!(cerne(&["g", "endpoint", "RegisterSale", "POST", "/sales"], &caixa));
    assert!(cerne(&["g", "endpoint", "Till", "GET", "/till"], &caixa));

    let caixa_cargo_toml = fs::read_to_string(caixa.join("Cargo.toml")).unwrap();

    assert!(caixa_cargo_toml.contains(r#"http = "rest""#));
    assert!(caixa_cargo_toml.contains(r#"features = ["axum"]"#));
    assert!(
        fs::read_to_string(caixa.join("src/main.rs"))
            .unwrap()
            .contains("axum::serve(listener, router(ports))")
    );

    // The query is the one part the actor's developer writes: here, it echoes the query string.
    let till_query = caixa.join("src/application/queries/till.rs");
    let till_query_written = fs::read_to_string(&till_query)
        .unwrap()
        .replace(r#"todo!("read the ports and build the Till read model")"#, "Ok(Till { total: self.total })");

    fs::write(&till_query, till_query_written).unwrap();
    fs::write(caixa.join("tests/rest.rs"), REST_TEST).unwrap();
    fs::write(
        caixa.join("Cargo.toml"),
        caixa_cargo_toml + "\n[dev-dependencies]\ntower = { version = \"0.5\", features = [\"util\"] }\n",
    )
    .unwrap();

    let rest_passed = cargo(&caixa, &workspace, &["clippy", "--all-targets", "--", "-D", "warnings"])
        && cargo(&caixa, &workspace, &["test"]);

    // --- cerne g http jsonrpc: the commands and queries already there get their method

    assert!(cerne(&["new", "balcao"], &tmp));

    let balcao = tmp.join("balcao");

    assert!(cerne(&["g", "command", "OpenTab", "table:u32"], &balcao));
    assert!(cerne(&["g", "command", "PrintBill", "table:u32", "--policy"], &balcao));
    assert!(cerne(&["g", "read_model", "Tab", "total:u64"], &balcao));
    assert!(cerne(&["g", "query", "Tab", "table:u32"], &balcao));
    assert!(cerne(&["g", "http", "jsonrpc"], &balcao));

    let balcao_rpc = fs::read_to_string(balcao.join("src/infrastructure/http/rpc.rs")).unwrap();

    assert!(balcao_rpc.contains(r#""open_tab" => methods.command::<OpenTabCommand>(request.params).await,"#));
    assert!(balcao_rpc.contains(r#""tab" => methods.query::<TabQuery>(request.params).await,"#));
    assert!(!balcao_rpc.contains("print_bill"), "a policy command has no actor to call it");
    assert!(
        fs::read_to_string(balcao.join("src/infrastructure/mod.rs"))
            .unwrap()
            .starts_with("pub mod http;")
    );

    // --- No database: the outbox in memory, then cerne g db sqlite ----------

    assert!(cerne(&["new", "nucleo", "--http", "rest"], &tmp));

    let nucleo = tmp.join("nucleo");

    assert!(!nucleo.join("migrations").exists());
    assert!(cerne(
        &[
            "g",
            "entity",
            "Order",
            "product:String",
            "quantity:u32",
            "status=Placed:Placed,Paid",
            "--aggregate"
        ],
        &nucleo
    ));
    assert!(
        !nucleo
            .join("src/infrastructure/sqlite_order_repository.rs")
            .exists(),
        "no database, no repository"
    );
    assert!(cerne(&["g", "command", "PlaceOrder", "product:String", "quantity:u32"], &nucleo));
    assert!(cerne(&["g", "command", "ShipOrder", "order_id:OrderId", "--policy"], &nucleo));
    assert!(cerne(&["g", "endpoint", "PlaceOrder", "POST", "/orders"], &nucleo));

    let nucleo_cargo_toml = fs::read_to_string(nucleo.join("Cargo.toml")).unwrap();

    assert!(nucleo_cargo_toml.contains("default-features = false"));
    assert!(!nucleo_cargo_toml.contains("sqlx"));

    let no_database_passed = cargo(&nucleo, &workspace, &["clippy", "--all-targets", "--", "-D", "warnings"]);

    assert!(!cerne(&["g", "db", "mysql"], &nucleo));
    assert!(cerne(&["g", "db", "sqlite"], &nucleo));
    assert!(!cerne(&["g", "db", "postgres"], &nucleo), "the project already has a database");
    assert!(
        nucleo
            .join("src/infrastructure/sqlite_order_repository.rs")
            .exists()
    );
    assert!(nucleo.join("migrations/1_create_cerne_outbox.sql").exists());
    assert!(
        !fs::read_to_string(nucleo.join("src/ports.rs"))
            .unwrap()
            .contains("InMemoryOutbox")
    );

    fs::write(nucleo.join("tests/repository.rs"), NUCLEO_REPOSITORY_TEST).unwrap();

    let database_added_passed = cargo(&nucleo, &workspace, &["clippy", "--all-targets", "--", "-D", "warnings"])
        && cargo(&nucleo, &workspace, &["test"]);

    fs::remove_dir_all(&tmp).unwrap();

    assert!(no_database_passed);
    assert!(database_added_passed);
    assert!(memory_and_jsonrpc_passed);
    assert!(postgres_and_rest_passed);
    assert!(sqlite_passed);
    assert!(rest_passed);
}

/// Runs cargo in a generated project, with the repository's own crate standing in for the published one. Without
/// `DATABASE_URL`: a SQLite project would read the Postgres of the CI job as its database.
fn cargo(project: &Path, workspace: &Path, args: &[&str]) -> bool {
    cargo_command(project, workspace, args)
        .env_remove("DATABASE_URL")
        .status()
        .unwrap()
        .success()
}

/// The same, with `DATABASE_URL` pointing at a Postgres.
fn cargo_on_postgres(project: &Path, workspace: &Path, args: &[&str], database_url: &str) -> bool {
    cargo_command(project, workspace, args)
        .env("DATABASE_URL", database_url)
        .status()
        .unwrap()
        .success()
}

fn cargo_command(project: &Path, workspace: &Path, args: &[&str]) -> Command {
    let manifest = project.join("Cargo.toml");
    let cerne_path = workspace.join("crates/cerne");
    let cargo_toml = fs::read_to_string(&manifest).unwrap().replace(
        concat!("version = \"", env!("CARGO_PKG_VERSION"), "\""),
        &format!("path = {:?}", cerne_path.display().to_string()),
    );

    fs::write(&manifest, cargo_toml).unwrap();
    fs::copy(workspace.join("Cargo.lock"), project.join("Cargo.lock")).unwrap();

    let mut command = Command::new(env::var("CARGO").unwrap_or("cargo".into()));

    command
        .args(args)
        .current_dir(project)
        .env("CARGO_TARGET_DIR", workspace.join("target/e2e"));

    command
}

// --- The tutorial (docs/<language>/tutorial.md) is a project that compiles -

/// The fenced blocks of a Markdown file, with the line before each one: `(marker, language, code)`.
fn fenced_blocks(markdown: &str) -> Vec<(String, String, String)> {
    let mut blocks = vec![];
    let mut previous = "";
    let mut lines = markdown.lines();

    while let Some(line) = lines.next() {
        if let Some(language) = line.strip_prefix("```") {
            let code: Vec<&str> = lines.by_ref().take_while(|l| *l != "```").collect();

            blocks.push((previous.to_string(), language.to_string(), code.join("\n") + "\n"));
        }

        previous = line;
    }

    blocks
}

#[test]
fn tutorial_builds_and_passes_its_tests() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let tmp: PathBuf = env::temp_dir().join(format!("cerne-tutorial-{}", std::process::id()));

    fs::create_dir_all(&tmp).unwrap();

    let tutorial = fs::read_to_string(workspace.join("docs/en/tutorial.md")).unwrap();
    let mut project = tmp.clone();

    for (marker, language, code) in fenced_blocks(&tutorial) {
        // --- The commands of the tutorial: cerne and cd ----------------------

        if language == "bash" {
            for line in code.lines() {
                if let Some(folder) = line.strip_prefix("cd ") {
                    project = project.join(folder);
                } else if let Some(args) = line.strip_prefix("cerne ") {
                    let args: Vec<&str> = args.split_whitespace().collect();

                    assert!(cerne(&args, &project), "the tutorial runs: {line}");
                }
            }
        }

        // --- The files as the CLI generated them ------------------------------

        if let Some(path) = marker
            .strip_prefix("<!-- generated: ")
            .and_then(|rest| rest.strip_suffix(" -->"))
        {
            let generated = fs::read_to_string(project.join(path)).unwrap();

            assert_eq!(generated, code, "the tutorial shows {path} as the CLI generates it");
        }

        // --- The files of the tutorial ---------------------------------------

        if let Some(path) = marker
            .strip_prefix("<!-- file: ")
            .and_then(|rest| rest.strip_suffix(" -->"))
        {
            fs::write(project.join(path), code).unwrap();
        }
    }

    let tutorial_passed = cargo(&project, &workspace, &["clippy", "--all-targets", "--", "-D", "warnings"])
        && cargo(&project, &workspace, &["test"]);

    fs::remove_dir_all(&tmp).unwrap();

    assert!(tutorial_passed);
}

#[test]
fn every_tutorial_translation_has_the_same_code() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let code = |translation: &str| -> Vec<(String, String)> {
        let tutorial = fs::read_to_string(workspace.join(format!("docs/{translation}/tutorial.md"))).unwrap();

        // The diagrams and the comments of the commands are translated; the code is not.
        fenced_blocks(&tutorial)
            .into_iter()
            .filter(|(_, language, _)| language != "mermaid")
            .map(|(_, language, code)| {
                let untranslated: Vec<&str> = code
                    .lines()
                    .filter(|line| language != "bash" || !line.starts_with('#'))
                    .collect();

                (language, untranslated.join("\n"))
            })
            .collect()
    };

    let english = code("en");

    assert_eq!(code("pt-BR"), english, "docs/pt-BR/tutorial.md");
    assert_eq!(code("es"), english, "docs/es/tutorial.md");
}
