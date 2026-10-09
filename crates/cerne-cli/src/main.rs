//! `cerne`: creates a project laid out like the Event Storming board and generates its post-its.

use minijinja::{Environment, Value, context};
use std::error::Error;
use std::path::Path;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{env, fs, process};

const USAGE: &str = "\
usage:
  cerne new <name> [--db memory|sqlite|postgres] [--http rest|jsonrpc]   (no --db: no database, the outbox in memory)
  cerne g entity <Name> [field:type ...] [field:Value1,Value2 ...] [field=Initial:Value1,Value2 ...] [id:type] [--aggregate]
  cerne g value_object <Name> field:type [field:type ...]
  cerne g event <Name> [field:type ...]
  cerne g command <Name> [field:type ...] [--policy]
  cerne g read_model <Name> [field:type ...]
  cerne g query <Name> [field:type ...]   (needs the read model <Name>)
  cerne g endpoint <Name> <GET|POST|PUT|PATCH|DELETE> </path>   (REST projects)
  cerne g http <rest|jsonrpc>   (projects created without --http)
  cerne g db <memory|sqlite|postgres>   (projects created without --db)
  cerne g port <Name>
  cerne g adapter <Name> <Port>";

/// What `cerne new` writes: one folder per layer, each `mod.rs` ready for the generators to append to.
const PROJECT: [(&str, &str); 17] = [
    ("Cargo.toml", include_str!("../templates/Cargo.toml.jinja")),
    (".gitignore", "/target\n*.db\n*.db-shm\n*.db-wal\n"),
    ("rustfmt.toml", include_str!("../templates/rustfmt.toml.jinja")),
    ("src/lib.rs", include_str!("../templates/lib.rs.jinja")),
    ("src/main.rs", MAIN),
    ("src/ports.rs", include_str!("../templates/ports.rs.jinja")),
    ("src/domain/mod.rs", "pub mod entities;\npub mod events;\npub mod value_objects;\n"),
    ("src/domain/entities/mod.rs", ""),
    ("src/domain/events/mod.rs", ""),
    ("src/domain/value_objects/mod.rs", ""),
    ("src/application/mod.rs", "pub mod commands;\npub mod ports;\npub mod queries;\npub mod read_models;\n"),
    ("src/application/commands/mod.rs", ""),
    ("src/application/ports/mod.rs", ""),
    ("src/application/queries/mod.rs", ""),
    ("src/application/read_models/mod.rs", ""),
    ("src/infrastructure/mod.rs", "{% if http %}pub mod http;\n{% endif %}"),
    ("tests/board.rs", include_str!("../templates/board.rs.jinja")),
];

const MAIN: &str = include_str!("../templates/main.rs.jinja");
const DATABASE_SETUP: &str = include_str!("../templates/database_setup.rs.jinja");
const OUTBOX_MIGRATION: (&str, &str) =
    ("migrations/1_create_cerne_outbox.sql", include_str!("../templates/outbox_migration.sql.jinja"));
const HTTP_MOD: &str = include_str!("../templates/http_mod.rs.jinja");
const RPC: &str = include_str!("../templates/rpc.rs.jinja");
const VALUE_OBJECT: &str = include_str!("../templates/value_object.rs.jinja");
const ENTITY: &str = include_str!("../templates/entity.rs.jinja");
const REPOSITORY: &str = include_str!("../templates/repository.rs.jinja");
const AGGREGATE_MIGRATION: &str = include_str!("../templates/aggregate_migration.sql.jinja");
const EVENT: &str = include_str!("../templates/event.rs.jinja");
const COMMAND: &str = include_str!("../templates/command.rs.jinja");
const READ_MODEL: &str = include_str!("../templates/read_model.rs.jinja");
const QUERY: &str = include_str!("../templates/query.rs.jinja");
const ENDPOINT_COMMAND: &str = include_str!("../templates/endpoint_command.rs.jinja");
const ENDPOINT_QUERY: &str = include_str!("../templates/endpoint_query.rs.jinja");
const PORT: &str = include_str!("../templates/port.rs.jinja");
const ADAPTER: &str = include_str!("../templates/adapter.rs.jinja");

/// The lines `cerne g` adds to files that already exist, right before (or after) a line that `cerne new` wrote.
const PORTS_STRUCT: &str = "pub outbox: Box<dyn Outbox<Ports>>,";
const PORTS_NEW: &str = "outbox: Box::new(";
const COMMAND_REGISTRY: &str = "CommandRegistry::new()";
const RPC_LAST_ARM: &str = "method => methods.not_found(method),";
const ROUTER_STATE: &str = ".with_state(ports)";

type CliResult = Result<(), Box<dyn Error>>;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();

    let result = match args.as_slice() {
        ["new", name, flags @ ..] => new_project(name, flags),
        ["g" | "generate", "endpoint", name, method, path] => generate_endpoint(name, method, path),
        ["g" | "generate", "adapter", name, port] => generate_adapter(name, port),
        ["g" | "generate", "http", http] => generate_http(http),
        ["g" | "generate", "db", db] => generate_db(db),
        ["g" | "generate", kind, name, rest @ ..] => generate(kind, name, rest),
        _ => Err(USAGE.into()),
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

// --- cerne new ---------------------------------------------------------------

fn new_project(name: &str, flags: &[&str]) -> CliResult {
    let db = flag(flags, "--db", &["memory", "sqlite", "postgres"])?.unwrap_or("");
    let http = flag(flags, "--http", &["rest", "jsonrpc"])?.unwrap_or("");
    let root = Path::new(name);

    if root.exists() {
        return Err(format!("{name} already exists").into());
    }

    let project = project_context(name, db, http)?;

    let mut files = PROJECT.to_vec();

    if !db.is_empty() {
        files.push(OUTBOX_MIGRATION);
    }

    files.extend(http_files(http));

    for (path, template) in files {
        let file = root.join(path);

        fs::create_dir_all(file.parent().unwrap())?;
        fs::write(&file, render(template, &project)?)?;
    }

    fn none(choice: &str) -> &str {
        if choice.is_empty() { "none" } else { choice }
    }

    println!("created {name} (database: {}, http: {})", none(db), none(http));

    Ok(())
}

/// What the templates of `cerne new` read: the name, the database and the HTTP of the project. Without a database
/// (`db` empty), the project depends on no adapter of `cerne`.
fn project_context(name: &str, db: &str, http: &str) -> Result<Value, minijinja::Error> {
    let module = if db == "postgres" { "postgres" } else { "sqlite" };
    let prefix = pascal_case(module);

    let cerne_features: Vec<String> = [(db == "postgres", "postgres"), (!http.is_empty(), "axum")]
        .into_iter()
        .filter(|(wanted, _)| *wanted)
        .map(|(_, feature)| format!("{feature:?}"))
        .collect();

    let database = format!("{prefix}Database");
    let crate_name = name.replace('-', "_");
    let database_setup = render(DATABASE_SETUP, &context! { db, database, crate_name })?;

    Ok(context! {
        name,
        crate_name,
        db,
        http,
        module,
        database,
        database_setup => database_setup.trim_end(),
        outbox => format!("{prefix}Outbox"),
        cerne_features => cerne_features.join(", "),
        cerne_version => env!("CARGO_PKG_VERSION"),
    })
}

/// The files of the HTTP layer: the router, and the JSON-RPC endpoint when the project speaks JSON-RPC.
fn http_files(http: &str) -> Vec<(&'static str, &'static str)> {
    match http {
        "rest" => vec![("src/infrastructure/http/mod.rs", HTTP_MOD)],
        "jsonrpc" => vec![
            ("src/infrastructure/http/mod.rs", HTTP_MOD),
            ("src/infrastructure/http/rpc.rs", RPC),
        ],
        _ => vec![],
    }
}

/// `--db sqlite` → `Some("sqlite")`, checked against the allowed values.
fn flag<'a>(flags: &[&'a str], name: &str, allowed: &[&str]) -> Result<Option<&'a str>, String> {
    let Some(position) = flags.iter().position(|flag| *flag == name) else {
        return Ok(None);
    };

    match flags.get(position + 1) {
        Some(value) if allowed.contains(value) => Ok(Some(value)),
        _ => Err(format!("{name} takes one of {}", allowed.join("|"))),
    }
}

// --- cerne g -----------------------------------------------------------------

/// What `cerne new` chose, read from `[package.metadata.cerne]` in the `Cargo.toml` of the project.
struct Project {
    name: String,
    db: String,
    module: &'static str,
    prefix: &'static str,
    http: String,
}

fn project() -> Result<Project, String> {
    let cargo_toml = fs::read_to_string("Cargo.toml")
        .map_err(|_| "Cargo.toml not found: run cerne g inside a cerne project".to_string())?;

    let value = |key: &str| {
        cargo_toml
            .lines()
            .find_map(|line| line.strip_prefix(&format!("{key} = \"")))
            .and_then(|rest| rest.strip_suffix('"'))
            .map(String::from)
    };

    let (module, prefix) = match value("db").as_deref() {
        Some("postgres") => ("postgres", "Postgres"),
        _ => ("sqlite", "Sqlite"),
    };

    Ok(Project {
        name: value("name").unwrap_or_default(),
        db: value("db").unwrap_or_default(),
        module,
        prefix,
        http: value("http").unwrap_or_default(),
    })
}

fn generate(kind: &str, name: &str, args: &[&str]) -> CliResult {
    if !is_pascal_case(name) {
        return Err(format!("{name} is not a PascalCase name, like Order").into());
    }

    let aggregate = args.contains(&"--aggregate");
    let policy = args.contains(&"--policy");

    if aggregate && kind != "entity" {
        return Err("--aggregate is only for entity".into());
    }

    if policy && kind != "command" {
        return Err("--policy is only for command".into());
    }

    let mut fields = parse_fields(args.iter().filter(|arg| !arg.starts_with("--")))?;
    let id_type = take_id_type(&mut fields);
    let uses = value_object_uses(&fields);

    let fields = fields
        .into_iter()
        .map(|(field_name, ty)| field(name, kind, field_name, ty))
        .collect::<Result<Vec<_>, _>>()?;

    let file = snake_case(name);

    match kind {
        "entity" => {
            let id_type = id_type.unwrap_or("u64");
            let id_field = context! { name => "value", ty => id_type };

            if aggregate && !is_integer(id_type) && id_type != "String" {
                return Err(format!(
                    "{name}: the repository of an aggregate handles integer and String ids, not {id_type}"
                )
                .into());
            }

            add_file(
                &format!("src/domain/value_objects/{file}_id.rs"),
                VALUE_OBJECT,
                context! { name => format!("{name}Id"), fields => vec![id_field] },
            )?;

            add_file(
                &format!("src/domain/entities/{file}.rs"),
                ENTITY,
                context! { name, file, fields, aggregate, uses },
            )?;

            if aggregate {
                add_repository_if_there_is_a_database(name, &file, id_type, &fields)?;
            }

            Ok(())
        }
        "value_object" if fields.is_empty() => Err("a value object needs at least one field, like value:u64".into()),
        "value_object" => {
            add_file(&format!("src/domain/value_objects/{file}.rs"), VALUE_OBJECT, context! { name, fields, uses })
        }
        "event" => add_file(&format!("src/domain/events/{file}.rs"), EVENT, context! { name, fields, uses }),
        "command" => {
            add_file(&format!("src/application/commands/{file}.rs"), COMMAND, context! { name, fields, policy, uses })?;

            let command = format!("{name}Command");
            let command_use = format!("use crate::application::commands::{file}::{command};");

            if policy {
                // A policy fires it: the outbox must be able to read it back.
                insert_lines(
                    "src/ports.rs",
                    &command_use,
                    COMMAND_REGISTRY,
                    &[&format!("    .register::<{command}>()")],
                    After,
                )
            } else {
                add_rpc_method(&file, &command_use, &format!("command::<{command}>"))
            }
        }
        "read_model" => {
            add_file(&format!("src/application/read_models/{file}.rs"), READ_MODEL, context! { name, fields, uses })
        }
        "query" if !Path::new(&format!("src/application/read_models/{file}.rs")).exists() => {
            Err(format!("the query {name}Query returns the read model {name}: run cerne g read_model {name} first")
                .into())
        }
        "query" => {
            add_file(&format!("src/application/queries/{file}.rs"), QUERY, context! { name, file, fields, uses })?;

            let query_use = format!("use crate::application::queries::{file}::{name}Query;");

            add_rpc_method(&file, &query_use, &format!("query::<{name}Query>"))
        }
        "port" => add_file(&format!("src/application/ports/{file}.rs"), PORT, context! { name }),
        _ => Err(format!("unknown generator {kind}\n{USAGE}").into()),
    }
}

/// With a database, the SQL repository of the aggregate; without one, only a note: `cerne g db` adds it later.
fn add_repository_if_there_is_a_database(name: &str, file: &str, id_type: &str, fields: &[Value]) -> CliResult {
    if project()?.db.is_empty() {
        println!("no database yet: {name} has no repository; cerne g db <memory|sqlite|postgres> adds it");

        return Ok(());
    }

    add_repository(name, file, id_type, fields)
}

/// The SQL repository of an aggregate, its migration, and its field in the `Ports`.
fn add_repository(name: &str, file: &str, id_type: &str, fields: &[Value]) -> CliResult {
    let project = project()?;
    let table = format!("{file}s");
    let repository = format!("{}{name}Repository", project.prefix);
    let repository_file = snake_case(&repository);
    let migration = format!("{}_create_{table}.sql", next_migration_version()?);

    let columns: Vec<Value> = fields.iter().map(|field| column(file, field)).collect();
    let names: Vec<String> = fields
        .iter()
        .map(|field| field.get_attr("name").unwrap().to_string())
        .collect();

    let insert_sql = if names.is_empty() {
        format!("INSERT INTO {table} DEFAULT VALUES RETURNING id")
    } else {
        let placeholders: Vec<String> = (1..=names.len()).map(|n| format!("${n}")).collect();

        format!(
            "INSERT INTO {table} ({})\n             VALUES ({}) RETURNING id",
            names.join(", "),
            placeholders.join(", ")
        )
    };

    let assignments: Vec<String> = names
        .iter()
        .enumerate()
        .map(|(position, name)| format!("{name} = ${}", position + 2))
        .collect();
    let update_sql = format!("UPDATE {table} SET {}\n             WHERE id = $1", assignments.join(", "));

    let id_integer = is_integer(id_type);
    let id_column = match (id_integer, project.module) {
        (true, "postgres") => "id BIGINT GENERATED BY DEFAULT AS IDENTITY PRIMARY KEY",
        (true, _) => "id INTEGER PRIMARY KEY",
        (false, _) => "id TEXT PRIMARY KEY",
    };
    let (id_bind, id_read) = if id_integer {
        (
            cast(&format!("{id_type}::from(id.clone())"), id_type, "i64"),
            cast(r#"column::<i64>(&row, "id")?"#, "i64", id_type),
        )
    } else {
        ("String::from(id.clone())".to_string(), r#"column::<String>(&row, "id")?"#.to_string())
    };

    let enums: Vec<&Value> = columns
        .iter()
        .filter(|column| column.get_attr("variants").is_ok_and(|v| !v.is_undefined()))
        .collect();
    let uses_infrastructure_error = !id_integer
        || !enums.is_empty()
        || columns
            .iter()
            .any(|column| column.get_attr("json").is_ok_and(|json| json.is_true()));

    let aggregate = context! {
        name, file, table, migration, columns => columns.clone(), enums, id_integer, id_ty => id_type,
        id_bind, id_read, insert_sql, update_sql, uses_infrastructure_error,
        prefix => project.prefix, module => project.module,
        database => format!("{}Database", project.prefix),
        fields => columns.clone(),
    };

    add_file(&format!("src/infrastructure/{repository_file}.rs"), REPOSITORY, aggregate.clone())?;

    let migration_file = format!("migrations/{migration}");

    fs::write(&migration_file, render(AGGREGATE_MIGRATION, &context! { table, id_column, fields => columns })?)?;

    println!("created {migration_file}");

    insert_lines(
        "src/ports.rs",
        &format!(
            "use crate::domain::entities::{file}::{name};\nuse crate::infrastructure::{repository_file}::{repository};"
        ),
        PORTS_STRUCT,
        &[&format!("pub {file}_repository: Box<dyn Repository<{name}>>,")],
        Before,
    )?;

    add_to_use("src/ports.rs", "cerne::application", "Repository")?;

    insert_lines(
        "src/ports.rs",
        "",
        PORTS_NEW,
        &[&format!("{file}_repository: Box::new({repository}::new(database.clone())),")],
        Before,
    )
}

/// How one field of an aggregate goes to its column and comes back: `qty: i32` is a `BIGINT`, bound as `i64`.
fn column(entity: &str, field: &Value) -> Value {
    let name = field.get_attr("name").unwrap().to_string();
    let ty = field.get_attr("ty").unwrap().to_string();
    let variants = field.get_attr("variants").unwrap_or_default();
    let value = format!("{entity}.{name}");
    let read = |as_ty: &str| format!(r#"column::<{as_ty}>(&row, "{name}")?"#);

    let (sql, bind, read, json, function) = if !variants.is_undefined() {
        let function = snake_case(&ty);

        ("TEXT", format!("{function}_name({value})"), format!("{function}_from(&{})?", read("String")), false, function)
    } else if is_integer(&ty) {
        ("BIGINT", cast(&value, &ty, "i64"), cast(&read("i64"), "i64", &ty), false, String::new())
    } else if ty == "f32" || ty == "f64" {
        ("DOUBLE PRECISION", cast(&value, &ty, "f64"), cast(&read("f64"), "f64", &ty), false, String::new())
    } else if ty == "bool" {
        ("BOOLEAN", value, read("bool"), false, String::new())
    } else if ty == "String" {
        ("TEXT", format!("{value}.clone()"), read("String"), false, String::new())
    } else {
        // Any other type (a value object, a struct) is stored as JSON: it must be `Serialize` and `Deserialize`.
        let infrastructure = "map_err(|error| InfrastructureError::from(anyhow::Error::from(error)))?";

        (
            "TEXT",
            format!("serde_json::to_string(&{value}).{infrastructure}"),
            format!("serde_json::from_str(&{}).{infrastructure}", read("String")),
            true,
            String::new(),
        )
    };

    context! { name, ty, variants, sql, bind, read, json, function }
}

/// `cerne g endpoint PlaceOrder POST /orders`: a REST route for the command (or, with `GET`, the query) `PlaceOrder`.
fn generate_endpoint(name: &str, method: &str, path: &str) -> CliResult {
    let project = project()?;

    if project.http != "rest" {
        return Err("cerne g endpoint is for REST projects (cerne new <name> --http rest); in JSON-RPC, cerne g command and cerne g query add the method".into());
    }

    if !path.starts_with('/') {
        return Err(format!("{path} is not a path, like /orders").into());
    }

    let file = snake_case(name);
    let routing = method.to_lowercase();

    let (template, post_it) = match method {
        "GET" => (ENDPOINT_QUERY, format!("src/application/queries/{file}.rs")),
        "POST" | "PUT" | "PATCH" | "DELETE" => (ENDPOINT_COMMAND, format!("src/application/commands/{file}.rs")),
        _ => return Err(format!("{method} is not one of GET|POST|PUT|PATCH|DELETE").into()),
    };

    if !Path::new(&post_it).exists() {
        let generator = if method == "GET" { "query" } else { "command" };

        return Err(format!("{post_it} not found: run cerne g {generator} {name} first").into());
    }

    add_file(&format!("src/infrastructure/http/{file}.rs"), template, context! { name, file, method, path })?;

    insert_lines(
        "src/infrastructure/http/mod.rs",
        "",
        ROUTER_STATE,
        &[&format!(".route({path:?}, axum::routing::{routing}({file}::{file}))")],
        Before,
    )
}

/// `cerne g http rest|jsonrpc`: the HTTP layer of a project created without `--http`, as `cerne new --http` writes
/// it. In JSON-RPC, every command and query an actor sends gets its method.
fn generate_http(http: &str) -> CliResult {
    if !["rest", "jsonrpc"].contains(&http) {
        return Err(format!("cerne g http takes rest|jsonrpc, not {http}").into());
    }

    let project = project()?;

    if !project.http.is_empty() {
        return Err(format!("the project already speaks {}", project.http).into());
    }

    // --- Cargo.toml: the choice, axum, and the feature axum of cerne ---------

    let cargo_toml = fs::read_to_string("Cargo.toml")?;
    let mut cargo_lines: Vec<String> = cargo_toml.lines().map(String::from).collect();

    let Some(cerne_line) = cargo_lines
        .iter()
        .position(|line| line.starts_with("cerne = ") && line.contains("features = ["))
    else {
        return Err(r#"Cargo.toml has no line cerne = { .., features = [..] }: add the feature "axum" by hand"#.into());
    };

    let cerne_line_without_features = cargo_lines[cerne_line].contains("features = []");
    let cerne_with_axum = if cerne_line_without_features {
        cargo_lines[cerne_line].replace("features = []", r#"features = ["axum"]"#)
    } else {
        cargo_lines[cerne_line].replace("features = [", r#"features = ["axum", "#)
    };

    cargo_lines[cerne_line] = cerne_with_axum;
    cargo_lines.insert(cerne_line, r#"axum = "0.8""#.to_string());

    for line in cargo_lines.iter_mut() {
        if line == r#"http = """# {
            *line = format!("http = {http:?}");
        }
    }

    fs::write("Cargo.toml", cargo_lines.join("\n") + "\n")?;

    println!("updated Cargo.toml");

    // --- The HTTP layer ------------------------------------------------------

    let without_http = project_context(&project.name, &project.db, "")?;
    let with_http = project_context(&project.name, &project.db, http)?;

    for (path, template) in http_files(http) {
        fs::create_dir_all(Path::new(path).parent().unwrap())?;
        fs::write(path, render(template, &with_http)?)?;

        println!("created {path}");
    }

    let infrastructure_mod = fs::read_to_string("src/infrastructure/mod.rs")?;

    fs::write("src/infrastructure/mod.rs", format!("pub mod http;\n{infrastructure_mod}"))?;

    // --- main.rs: rewritten only if it is still the one cerne new wrote -------

    let main_rs = fs::read_to_string("src/main.rs")?;
    let main_is_untouched = main_rs == render(MAIN, &without_http)?;

    if main_is_untouched {
        fs::write("src/main.rs", render(MAIN, &with_http)?)?;

        println!("updated src/main.rs");
    } else {
        println!(
            "src/main.rs changed since cerne new: serve the router by hand, as in\n\n{}",
            render(MAIN, &with_http)?
        );
    }

    // --- JSON-RPC: one method per command and query of an actor --------------

    let ports_rs = fs::read_to_string("src/ports.rs")?;

    for file in post_it_files("src/application/commands")? {
        let command = format!("{}Command", pascal_case(&file));
        let fired_by_a_policy = ports_rs.contains(&format!("register::<{command}>()"));

        if !fired_by_a_policy {
            let command_use = format!("use crate::application::commands::{file}::{command};");

            add_rpc_method(&file, &command_use, &format!("command::<{command}>"))?;
        }
    }

    for file in post_it_files("src/application/queries")? {
        let query = format!("{}Query", pascal_case(&file));
        let query_use = format!("use crate::application::queries::{file}::{query};");

        add_rpc_method(&file, &query_use, &format!("query::<{query}>"))?;
    }

    Ok(())
}

/// The lines a project without a database has in `ports.rs` and `main.rs`, which `cerne g db` swaps for the database.
const NO_DATABASE_PORTS_DOC: &str = "/// No database yet (`cerne g db` adds one): the outbox lives in memory, and a transaction is only the same ports. If
/// the process dies, the commands the policies fired and that did not run yet are lost.";
const DATABASE_PORTS_DOC: &str =
    "/// The repositories and the outbox live in the database, so they follow its transaction. External systems do not:
/// `begin` hands the same adapters to the new ports.";
const NO_DATABASE_SETUP: &str =
    "    // No database yet (`cerne g db` adds one): the commands of the policies wait in memory.
    let in_memory_outbox = InMemoryOutbox::new();";

/// `cerne g db memory|sqlite|postgres`: the database of a project created without `--db`, as `cerne new --db` writes
/// it: `sqlx` and the adapters of `cerne`, the outbox table, the `Ports` on the database, and the SQL repository of
/// every aggregate that already exists.
fn generate_db(db: &str) -> CliResult {
    if !["memory", "sqlite", "postgres"].contains(&db) {
        return Err(format!("cerne g db takes memory|sqlite|postgres, not {db}").into());
    }

    let project = project()?;

    if !project.db.is_empty() {
        return Err(format!("the project already has the database {}", project.db).into());
    }

    let with_db = project_context(&project.name, db, &project.http)?;
    let module = with_db.get_attr("module")?.to_string();
    let database = with_db.get_attr("database")?.to_string();
    let outbox = with_db.get_attr("outbox")?.to_string();

    // --- Cargo.toml: the choice, sqlx, and the adapters of cerne -------------

    let cargo_toml = fs::read_to_string("Cargo.toml")?;
    let mut cargo_lines: Vec<String> = cargo_toml.lines().map(String::from).collect();

    let Some(cerne_line) = cargo_lines
        .iter()
        .position(|line| line.starts_with("cerne = ") && line.contains(", default-features = false"))
    else {
        return Err(
            "Cargo.toml has no line cerne = { .., default-features = false, .. }: add the database by hand".into()
        );
    };

    cargo_lines[cerne_line] = if db == "postgres" {
        let cerne_line_without_features = cargo_lines[cerne_line].contains("features = []");

        if cerne_line_without_features {
            cargo_lines[cerne_line].replace("features = []", r#"features = ["postgres"]"#)
        } else {
            cargo_lines[cerne_line].replace("features = [", r#"features = ["postgres", "#)
        }
    } else {
        cargo_lines[cerne_line].replace(", default-features = false", "")
    };

    let sqlx_line = format!(
        r#"sqlx = {{ version = "0.8", default-features = false, features = ["runtime-tokio", "{module}", "macros", "migrate"] }}"#
    );
    let tokio_line = cargo_lines
        .iter()
        .position(|line| line.starts_with("tokio = "))
        .unwrap_or(cerne_line + 1);

    cargo_lines.insert(tokio_line, sqlx_line);

    for line in cargo_lines.iter_mut() {
        if line == r#"db = """# {
            *line = format!("db = {db:?}");
        }
    }

    fs::write("Cargo.toml", cargo_lines.join("\n") + "\n")?;

    println!("updated Cargo.toml");

    // --- The outbox table ----------------------------------------------------

    let (migration, template) = OUTBOX_MIGRATION;

    fs::create_dir_all("migrations")?;
    fs::write(migration, render(template, &with_db)?)?;

    println!("created {migration}");

    // --- ports.rs: the in-memory outbox becomes the database -----------------

    let ports_rs = fs::read_to_string("src/ports.rs")?
        .replace(NO_DATABASE_PORTS_DOC, DATABASE_PORTS_DOC)
        .replace("Box::new(in_memory_outbox.clone())", &format!("Box::new({outbox}::new(database.clone()))"))
        .replace("in_memory_outbox: InMemoryOutbox", &format!("database: {database}"))
        .replace("in_memory_outbox", "database")
        .replace("InMemoryOutbox, ", "");

    if ports_rs.contains("InMemoryOutbox") {
        return Err("src/ports.rs still uses InMemoryOutbox: swap it for the database by hand".into());
    }

    let database_use = format!("use cerne::{module}::{{{database}, {outbox}}};");

    fs::write("src/ports.rs", format!("{database_use}\n{ports_rs}"))?;
    rustfmt(Path::new("src/ports.rs"));

    println!("updated src/ports.rs");

    // --- main.rs: the database instead of the in-memory outbox ---------------

    let main_rs = fs::read_to_string("src/main.rs")?;
    let database_setup = with_db.get_attr("database_setup")?.to_string();

    if main_rs.contains(NO_DATABASE_SETUP) {
        let main_rs = main_rs
            .replace(NO_DATABASE_SETUP, &database_setup)
            .replace("Ports::new(in_memory_outbox", "Ports::new(database")
            .replace("{InMemoryOutbox, OutboxPolicyProcessor}", "OutboxPolicyProcessor");

        fs::write("src/main.rs", format!("use cerne::{module}::{database};\n{main_rs}"))?;
        rustfmt(Path::new("src/main.rs"));

        println!("updated src/main.rs");
    } else {
        println!(
            "src/main.rs changed since cerne new: build the Ports on the database by hand, as in\n\n{database_setup}\n"
        );
    }

    // --- The SQL repository of every aggregate that already exists -----------

    for aggregate in existing_aggregates()? {
        add_repository(&aggregate.name, &aggregate.file, &aggregate.id_type, &aggregate.fields)?;
    }

    println!("tests that build Ports::new(InMemoryOutbox::new()) now need the database, as src/main.rs builds it");

    Ok(())
}

/// An aggregate read back from its file, as `cerne g entity --aggregate` wrote it.
struct ExistingAggregate {
    name: String,
    file: String,
    id_type: String,
    fields: Vec<Value>,
}

/// Every aggregate in `src/domain/entities/`: what `add_repository` needs to give it a SQL repository.
fn existing_aggregates() -> Result<Vec<ExistingAggregate>, Box<dyn Error>> {
    let mut aggregates = vec![];

    for file in post_it_files("src/domain/entities")? {
        let entity = fs::read_to_string(format!("src/domain/entities/{file}.rs"))?;

        let Some(name) = entity
            .lines()
            .skip_while(|line| line.trim() != "#[aggregate]")
            .find_map(|line| {
                line.strip_prefix("pub struct ")
                    .and_then(|rest| rest.split_whitespace().next())
            })
        else {
            continue;
        };

        let id_file = fs::read_to_string(format!("src/domain/value_objects/{file}_id.rs"))?;
        let id_type = id_file
            .lines()
            .find_map(|line| line.strip_prefix(&format!("pub struct {name}Id(")))
            .and_then(|rest| rest.strip_suffix(");"))
            .ok_or(format!("src/domain/value_objects/{file}_id.rs has no pub struct {name}Id(..);"))?;

        let fields: Vec<Value> = struct_body(&entity, &format!("pub struct {name} {{"))
            .iter()
            .filter_map(|line| {
                line.strip_prefix("pub ")?
                    .strip_suffix(',')?
                    .split_once(": ")
            })
            .filter(|(field_name, _)| *field_name != "id")
            .map(|(field_name, ty)| {
                let variants: Vec<String> = struct_body(&entity, &format!("pub enum {ty} {{"))
                    .iter()
                    .filter(|line| !line.starts_with("#["))
                    .map(|variant| variant.trim_end_matches(',').to_string())
                    .collect();

                if variants.is_empty() {
                    context! { name => field_name, ty }
                } else {
                    context! { name => field_name, ty, variants }
                }
            })
            .collect();

        aggregates.push(ExistingAggregate {
            name: name.to_string(),
            file,
            id_type: id_type.to_string(),
            fields,
        });
    }

    Ok(aggregates)
}

/// The trimmed lines between `opening` and the next `}`: the fields of a struct or the variants of an enum.
fn struct_body<'a>(source: &'a str, opening: &str) -> Vec<&'a str> {
    source
        .lines()
        .skip_while(|line| line.trim() != opening)
        .skip(1)
        .map(str::trim)
        .take_while(|line| *line != "}")
        .collect()
}

/// The modules of a folder, without `mod.rs`, in alphabetical order: `place_order`, `ship_order`.
fn post_it_files(folder: &str) -> Result<Vec<String>, Box<dyn Error>> {
    let mut files: Vec<String> = fs::read_dir(folder)?
        .filter_map(|entry| {
            let name = entry.ok()?.file_name().to_string_lossy().to_string();

            name.strip_suffix(".rs")
                .filter(|module| *module != "mod")
                .map(String::from)
        })
        .collect();

    files.sort();

    Ok(files)
}

/// `cerne g adapter SmtpNotifier Notifier`: a struct in `infrastructure/` that implements the port `Notifier`.
fn generate_adapter(name: &str, port: &str) -> CliResult {
    if !is_pascal_case(name) || !is_pascal_case(port) {
        return Err(format!("{name} and {port} must be PascalCase names, like SmtpNotifier Notifier").into());
    }

    let port_file = snake_case(port);

    if !Path::new(&format!("src/application/ports/{port_file}.rs")).exists() {
        return Err(format!("the port {port} does not exist: run cerne g port {port} first").into());
    }

    add_file(&format!("src/infrastructure/{}.rs", snake_case(name)), ADAPTER, context! { name, port, port_file })
}

/// In a JSON-RPC project, the arm `"place_order" => methods.command::<PlaceOrderCommand>(..)`.
fn add_rpc_method(file: &str, post_it_use: &str, call: &str) -> CliResult {
    if project()?.http != "jsonrpc" {
        return Ok(());
    }

    let rpc = "src/infrastructure/http/rpc.rs";

    insert_lines(
        rpc,
        post_it_use,
        RPC_LAST_ARM,
        &[&format!("{file:?} => methods.{call}(request.params).await,")],
        Before,
    )?;

    // With an arm of its own, the `match` no longer needs the allow `cerne new` wrote.
    let content = fs::read_to_string(rpc)?;
    let content: Vec<&str> = content
        .lines()
        .filter(|line| !line.contains("#[allow(clippy::match_single_binding)]"))
        .collect();

    fs::write(rpc, content.join("\n") + "\n")?;

    Ok(())
}

// --- Files -------------------------------------------------------------------

/// Writes the post-it and appends `pub mod <file>;` to the `mod.rs` next to it, so it compiles right away.
fn add_file(path: &str, template: &str, post_it: Value) -> CliResult {
    let file = Path::new(path);
    let mod_rs = file.with_file_name("mod.rs");

    if !mod_rs.exists() {
        return Err(format!("{} not found: run cerne g inside a cerne project", mod_rs.display()).into());
    }

    if file.exists() {
        return Err(format!("{path} already exists").into());
    }

    fs::write(file, render(template, &post_it)?)?;
    rustfmt(file);

    let module = file.file_stem().unwrap().to_string_lossy();
    let mut mod_rs_content = fs::read_to_string(&mod_rs)?;

    if !mod_rs_content.is_empty() && !mod_rs_content.ends_with('\n') {
        mod_rs_content.push('\n');
    }

    mod_rs_content.push_str(&format!("pub mod {module};\n"));
    fs::write(&mod_rs, mod_rs_content)?;

    println!("created {path}");

    Ok(())
}

#[derive(PartialEq)]
enum Position {
    Before,
    After,
}

use Position::{After, Before};

/// Adds `lines` right before (or after) the line that contains `anchor`, with its indentation, and the `use` lines at
/// the top. The anchor is a line `cerne new` wrote; nothing else in the file changes.
fn insert_lines(path: &str, uses: &str, anchor: &str, lines: &[&str], position: Position) -> CliResult {
    let content = fs::read_to_string(path).map_err(|_| format!("{path} not found"))?;
    let mut file_lines: Vec<String> = content.lines().map(String::from).collect();

    let Some(at) = file_lines.iter().position(|line| line.contains(anchor)) else {
        return Err(format!("{path} has no line with {anchor:?}: add {lines:?} by hand").into());
    };

    let indentation: String = file_lines[at]
        .chars()
        .take_while(|c| c.is_whitespace())
        .collect();
    let at = if position == After { at + 1 } else { at };

    for line in lines.iter().rev() {
        file_lines.insert(at, format!("{indentation}{}", line.trim_start()));
    }

    for use_line in uses.lines().rev() {
        if !file_lines.iter().any(|line| line == use_line) {
            file_lines.insert(0, use_line.to_string());
        }
    }

    fs::write(path, file_lines.join("\n") + "\n")?;
    rustfmt(Path::new(path));

    println!("updated {path}");

    Ok(())
}

/// `use cerne::application::{CommandRegistry, Outbox};` + `Repository` →
/// `use cerne::application::{CommandRegistry, Outbox, Repository};`: one `use` per path, the names in order.
fn add_to_use(path: &str, module: &str, name: &str) -> CliResult {
    let content = fs::read_to_string(path).map_err(|_| format!("{path} not found"))?;
    let opening = format!("use {module}::{{");

    let Some(use_line) = content.lines().find(|line| line.starts_with(&opening)) else {
        fs::write(path, format!("use {module}::{name};\n{content}"))?;

        return Ok(());
    };

    let mut names: Vec<&str> = use_line[opening.len()..]
        .trim_end_matches("};")
        .split(',')
        .map(str::trim)
        .filter(|existing| !existing.is_empty())
        .collect();

    if names.contains(&name) {
        return Ok(());
    }

    names.push(name);
    names.sort_by_key(|name| name.to_lowercase());

    let merged_use_line = format!("{opening}{}}};", names.join(", "));

    fs::write(path, content.replacen(use_line, &merged_use_line, 1))?;

    Ok(())
}

/// The version of a new migration: the current second, or one more than the last migration, whichever is bigger.
fn next_migration_version() -> Result<u64, Box<dyn Error>> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

    let last = fs::read_dir("migrations")
        .map_err(|_| "migrations/ not found: run cerne g inside a cerne project")?
        .filter_map(|entry| {
            let name = entry.ok()?.file_name().to_string_lossy().to_string();

            name.split('_').next()?.parse::<u64>().ok()
        })
        .max()
        .unwrap_or(0);

    Ok(now.max(last + 1))
}

fn rustfmt(file: &Path) {
    // Best effort: without rustfmt the code is still valid, only less tidy.
    let _ = process::Command::new("rustfmt")
        .args(["--edition", "2024"])
        .arg(file)
        .status();
}

// --- Helpers -----------------------------------------------------------------

fn render(template: &str, data: &Value) -> Result<String, minijinja::Error> {
    let mut environment = Environment::new();

    environment.set_trim_blocks(true);
    environment.set_lstrip_blocks(true);
    environment.set_keep_trailing_newline(true);

    environment.render_str(template, data)
}

/// `qty:i32 id:u64` → `[(qty, i32), (id, u64)]`, in the order given.
fn parse_fields<'a>(args: impl Iterator<Item = &'a &'a str>) -> Result<Vec<(&'a str, &'a str)>, String> {
    args.map(|arg| match arg.split_once(':') {
        Some((name, ty)) if !name.is_empty() && !ty.is_empty() => Ok((name, ty)),
        _ => Err(format!("{arg} is not a field: use name:type, like qty:i32")),
    })
    .collect()
}

/// Removes the `id:<type>` field, if there is one, and gives back its type: the id becomes a value object.
fn take_id_type<'a>(fields: &mut Vec<(&'a str, &'a str)>) -> Option<&'a str> {
    let position = fields.iter().position(|(name, _)| *name == "id")?;

    Some(fields.remove(position).1)
}

/// `qty:i32` is a plain field. In `Product`, `kind:Physical,Digital` is the enum `ProductKind`, chosen by whoever
/// creates the product. In `Order`, `status=Pending:Pending,Accepted` is the enum `OrderStatus`, and every new
/// `Order` starts as `Pending`.
fn field(owner: &str, kind: &str, name: &str, ty: &str) -> Result<Value, String> {
    let (name, initial) = match name.split_once('=') {
        Some((name, initial)) => (name, Some(initial)),
        None => (name, None),
    };

    let is_enum = ty.contains(',');

    if !is_enum && initial.is_none() {
        return Ok(context! { name, ty });
    }

    if !is_enum {
        return Err(format!("{name}: = only works with enum values, like status=Pending:Pending,Accepted"));
    }

    if kind != "entity" {
        return Err(format!("{name}:{ty}: enum values only work in cerne g entity"));
    }

    let variants: Vec<&str> = ty.split(',').collect();

    if !variants.iter().all(|variant| is_pascal_case(variant)) {
        return Err(format!("{name}:{ty}: every value must be PascalCase, like status:Pending,Accepted"));
    }

    if initial.is_some_and(|initial| !variants.contains(&initial)) {
        return Err(format!("{name}: the initial value must be one of {ty}, like status=Pending:Pending,Accepted"));
    }

    let title = pascal_case(name);
    let section = format!("// --- {title} {}", "-".repeat(80 - 8 - title.len()));

    Ok(context! { name, ty => format!("{owner}{title}"), variants, initial, section })
}

/// `order_id:OrderId`, when `OrderId` is a value object of the project: the `use` the post-it needs for it.
fn value_object_uses(fields: &[(&str, &str)]) -> Vec<String> {
    fields
        .iter()
        .map(|(_, ty)| *ty)
        .filter(|ty| is_pascal_case(ty))
        .filter(|ty| Path::new(&format!("src/domain/value_objects/{}.rs", snake_case(ty))).exists())
        .map(|ty| format!("use crate::domain::value_objects::{}::{ty};", snake_case(ty)))
        .collect()
}

fn is_integer(ty: &str) -> bool {
    matches!(ty, "i8" | "i16" | "i32" | "i64" | "isize" | "u8" | "u16" | "u32" | "u64" | "usize")
}

/// `value as i64`, unless the value already is an `i64` (clippy refuses a cast to the same type).
fn cast(value: &str, from: &str, to: &str) -> String {
    if from == to { value.to_string() } else { format!("{value} as {to}") }
}

fn is_pascal_case(name: &str) -> bool {
    name.starts_with(|letter: char| letter.is_ascii_uppercase())
        && name.chars().all(|letter| letter.is_ascii_alphanumeric())
}

/// `payment_status` → `PaymentStatus`.
fn pascal_case(name: &str) -> String {
    name.split('_')
        .flat_map(|word| {
            let mut letters = word.chars();

            letters
                .next()
                .into_iter()
                .flat_map(char::to_uppercase)
                .chain(letters)
        })
        .collect()
}

/// `OrderItem` → `order_item`.
fn snake_case(name: &str) -> String {
    let mut snake = String::new();

    for (position, letter) in name.chars().enumerate() {
        if letter.is_uppercase() && position > 0 {
            snake.push('_');
        }

        snake.extend(letter.to_lowercase());
    }

    snake
}
