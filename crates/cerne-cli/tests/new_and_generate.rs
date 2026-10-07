use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};

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

    assert!(cerne(&["new", "loja"], &tmp));
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

    // --- cargo clippy: the repository's own crate stands in for the Git dependency

    let manifest = project.join("Cargo.toml");
    let cerne_path = workspace.join("crates/cerne");
    let cargo_toml = fs::read_to_string(&manifest).unwrap().replace(
        r#"git = "https://github.com/ecdesa-labs/cerne""#,
        &format!("path = {:?}", cerne_path.display().to_string()),
    );

    fs::write(&manifest, cargo_toml).unwrap();
    fs::copy(workspace.join("Cargo.lock"), project.join("Cargo.lock")).unwrap();

    let clippy_passed = Command::new(env::var("CARGO").unwrap_or("cargo".into()))
        .args(["clippy", "--all-targets", "--", "-D", "warnings"])
        .current_dir(&project)
        .env("CARGO_TARGET_DIR", workspace.join("target/e2e"))
        .status()
        .unwrap()
        .success();

    fs::remove_dir_all(&tmp).unwrap();

    assert!(clippy_passed);
}
