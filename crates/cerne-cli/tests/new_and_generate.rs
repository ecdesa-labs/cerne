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
    assert!(!cerne(&["new", "loja"], &tmp), "new must not overwrite a project");
    assert!(!cerne(&["new", "bad", "--db", "sqlite"], &tmp), "new takes no flags");

    let project = tmp.join("loja");

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
            "id:String",
            "--aggregate"
        ],
        &project
    ));
    assert!(cerne(&["g", "value_object", "Amount", "value:u64"], &project));
    assert!(cerne(&["g", "value_object", "Money", "amount:u64", "currency:String"], &project));
    assert!(cerne(&["g", "event", "OrderPlaced", "order_id:OrderId"], &project));
    assert!(cerne(&["g", "event", "Pinged"], &project));
    assert!(cerne(&["g", "command", "PlaceOrder", "order_id:OrderId", "qty:i32"], &project));
    assert!(cerne(&["g", "command", "ShipOrder"], &project));
    assert!(cerne(&["g", "port", "Notifier"], &project));
    assert!(cerne(&["g", "adapter", "SmtpNotifier", "Notifier"], &project));
    assert!(!cerne(&["g", "adapter", "Smtp", "Mailer"], &project));

    assert!(!cerne(&["g", "query", "OrderSummary", "order_id:u64"], &project), "a query needs its read model first");
    assert!(cerne(&["g", "read_model", "OrderSummary", "order_id:u64", "total:u64"], &project));
    assert!(cerne(&["g", "query", "OrderSummary", "order_id:u64"], &project));

    assert!(!cerne(&["g", "entity", "Order"], &project), "g must not overwrite a post-it");
    assert!(!cerne(&["g", "command", "Charge", "--policy"], &project), "a command has no --policy");
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

    // --- The port of each aggregate's repository, and its adapter still to write

    let composition_root = fs::read_to_string(project.join("src/composition_root.rs")).unwrap();

    assert!(composition_root.contains("pub order_repository: Box<dyn Repository<Order, Transaction>>,"));
    assert!(composition_root.contains("payment_repository: constructor.payment_repository,"));

    let main_rs = fs::read_to_string(project.join("src/main.rs")).unwrap();

    assert!(main_rs.contains("let order_repository = order_repository_adapter();"));
    assert!(main_rs.contains("fn payment_repository_adapter() -> Box<dyn Repository<Payment, Transaction>> {"));
    assert!(main_rs.contains(
        "        database,\n        order_repository,\n        payment_repository,\n        event_outbox,\n"
    ));

    // --- cargo clippy, on the project as the CLI left it ---------------------

    let project_passed = cargo(&project, &workspace, &["clippy", "--all-targets", "--", "-D", "warnings"]);

    fs::remove_dir_all(&tmp).unwrap();

    assert!(project_passed);
}

/// Runs cargo in a generated project, with the repository's own crate standing in for the published one.
fn cargo(project: &Path, workspace: &Path, args: &[&str]) -> bool {
    let manifest = project.join("Cargo.toml");
    let cerne_path = workspace.join("crates/cerne");
    let cargo_toml = fs::read_to_string(&manifest).unwrap().replace(
        concat!("cerne = { version = \"", env!("CARGO_PKG_VERSION"), "\" }"),
        &format!("cerne = {{ path = {:?} }}", cerne_path.display().to_string()),
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
