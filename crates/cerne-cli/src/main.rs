//! `cerne`: creates a project laid out like the Event Storming board and generates its post-its.

use minijinja::{Environment, Value, context};
use std::error::Error;
use std::path::Path;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{env, fs, process};

const USAGE: &str = "\
usage:
  cerne new <name> [--db memory|sqlite|postgres] [--http rest|jsonrpc]
  cerne g entity <Name> [field:type ...] [field:Value1,Value2 ...] [field=Initial:Value1,Value2 ...] [id:type] [--aggregate]
  cerne g value_object <Name> field:type [field:type ...]
  cerne g event <Name> [field:type ...]
  cerne g command <Name> [field:type ...] [--policy]
  cerne g read_model <Name> [field:type ...]
  cerne g query <Name> [field:type ...]   (needs the read model <Name>)
  cerne g endpoint <Name> <GET|POST|PUT|PATCH|DELETE> </path>   (REST projects)
  cerne g port <Name>
  cerne g adapter <Name> <Port>";

/// What `cerne new` writes: the layers of D11, each `mod.rs` ready for the generators to append to (D12).
const PROJECT: [(&str, &str); 17] = [
    ("Cargo.toml", include_str!("../templates/Cargo.toml.jinja")),
    (".gitignore", "/target\n*.db\n*.db-shm\n*.db-wal\n"),
    ("src/lib.rs", include_str!("../templates/lib.rs.jinja")),
    ("src/main.rs", include_str!("../templates/main.rs.jinja")),
    ("src/ports.rs", include_str!("../templates/ports.rs.jinja")),
    (
        "src/domain/mod.rs",
        "pub mod entities;\npub mod events;\npub mod value_objects;\n",
    ),
    ("src/domain/entities/mod.rs", ""),
    ("src/domain/events/mod.rs", ""),
    ("src/domain/value_objects/mod.rs", ""),
    (
        "src/application/mod.rs",
        "pub mod commands;\npub mod ports;\npub mod queries;\npub mod read_models;\n",
    ),
    ("src/application/commands/mod.rs", ""),
    ("src/application/ports/mod.rs", ""),
    ("src/application/queries/mod.rs", ""),
    ("src/application/read_models/mod.rs", ""),
    (
        "src/infrastructure/mod.rs",
        "{% if http %}pub mod http;\n{% endif %}",
    ),
    (
        "migrations/1_create_cerne_outbox.sql",
        include_str!("../templates/outbox_migration.sql.jinja"),
    ),
    (
        "tests/board.rs",
        include_str!("../templates/board.rs.jinja"),
    ),
];

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
    let db = flag(flags, "--db", &["memory", "sqlite", "postgres"])?.unwrap_or("memory");
    let http = flag(flags, "--http", &["rest", "jsonrpc"])?.unwrap_or("");
    let root = Path::new(name);

    if root.exists() {
        return Err(format!("{name} already exists").into());
    }

    let module = if db == "postgres" {
        "postgres"
    } else {
        "sqlite"
    };
    let prefix = pascal_case(module);

    let cerne_features: Vec<String> = [(db == "postgres", "postgres"), (!http.is_empty(), "axum")]
        .into_iter()
        .filter(|(wanted, _)| *wanted)
        .map(|(_, feature)| format!("{feature:?}"))
        .collect();

    let project = context! {
        name,
        crate_name => name.replace('-', "_"),
        db,
        http,
        module,
        database => format!("{prefix}Database"),
        outbox => format!("{prefix}Outbox"),
        cerne_features => cerne_features.join(", "),
    };

    let mut files = PROJECT.to_vec();

    if !http.is_empty() {
        files.push(("src/infrastructure/http/mod.rs", HTTP_MOD));
    }

    if http == "jsonrpc" {
        files.push(("src/infrastructure/http/rpc.rs", RPC));
    }

    for (path, template) in files {
        let file = root.join(path);

        fs::create_dir_all(file.parent().unwrap())?;
        fs::write(&file, render(template, &project)?)?;
    }

    println!(
        "created {name} (database: {db}, http: {})",
        if http.is_empty() { "none" } else { http }
    );

    Ok(())
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
                context! { name, file, fields, aggregate },
            )?;

            if aggregate {
                add_repository(name, &file, id_type, &fields)?;
            }

            Ok(())
        }
        "value_object" if fields.is_empty() => {
            Err("a value object needs at least one field, like value:u64".into())
        }
        "value_object" => add_file(
            &format!("src/domain/value_objects/{file}.rs"),
            VALUE_OBJECT,
            context! { name, fields },
        ),
        "event" => add_file(
            &format!("src/domain/events/{file}.rs"),
            EVENT,
            context! { name, fields },
        ),
        "command" => {
            add_file(
                &format!("src/application/commands/{file}.rs"),
                COMMAND,
                context! { name, fields, policy },
            )?;

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
        "read_model" => add_file(
            &format!("src/application/read_models/{file}.rs"),
            READ_MODEL,
            context! { name, fields },
        ),
        "query" if !Path::new(&format!("src/application/read_models/{file}.rs")).exists() => Err(format!(
            "the query {name}Query returns the read model {name}: run cerne g read_model {name} first"
        )
        .into()),
        "query" => {
            add_file(
                &format!("src/application/queries/{file}.rs"),
                QUERY,
                context! { name, file, fields },
            )?;

            let query_use = format!("use crate::application::queries::{file}::{name}Query;");

            add_rpc_method(&file, &query_use, &format!("query::<{name}Query>"))
        }
        "port" => add_file(
            &format!("src/application/ports/{file}.rs"),
            PORT,
            context! { name },
        ),
        _ => Err(format!("unknown generator {kind}\n{USAGE}").into()),
    }
}

/// The SQL repository of an aggregate, its migration, and its field in the `Ports` (D31).
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
    let update_sql = format!(
        "UPDATE {table} SET {}\n             WHERE id = $1",
        assignments.join(", ")
    );

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
        (
            "String::from(id.clone())".to_string(),
            r#"column::<String>(&row, "id")?"#.to_string(),
        )
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

    add_file(
        &format!("src/infrastructure/{repository_file}.rs"),
        REPOSITORY,
        aggregate.clone(),
    )?;

    let migration_file = format!("migrations/{migration}");

    fs::write(
        &migration_file,
        render(
            AGGREGATE_MIGRATION,
            &context! { table, id_column, fields => columns },
        )?,
    )?;

    println!("created {migration_file}");

    insert_lines(
        "src/ports.rs",
        &format!(
            "use crate::domain::entities::{file}::{name};\nuse crate::infrastructure::{repository_file}::{repository};"
        ),
        PORTS_STRUCT,
        &[&format!("pub {table}: Box<dyn Repository<{name}>>,")],
        Before,
    )?;

    insert_lines(
        "src/ports.rs",
        "use cerne::application::Repository;",
        PORTS_NEW,
        &[&format!(
            "{table}: Box::new({repository}::new(database.clone())),"
        )],
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

        (
            "TEXT",
            format!("{function}_name({value})"),
            format!("{function}_from(&{})?", read("String")),
            false,
            function,
        )
    } else if is_integer(&ty) {
        (
            "BIGINT",
            cast(&value, &ty, "i64"),
            cast(&read("i64"), "i64", &ty),
            false,
            String::new(),
        )
    } else if ty == "f32" || ty == "f64" {
        (
            "DOUBLE PRECISION",
            cast(&value, &ty, "f64"),
            cast(&read("f64"), "f64", &ty),
            false,
            String::new(),
        )
    } else if ty == "bool" {
        ("BOOLEAN", value, read("bool"), false, String::new())
    } else if ty == "String" {
        (
            "TEXT",
            format!("{value}.clone()"),
            read("String"),
            false,
            String::new(),
        )
    } else {
        // Any other type (a value object, a struct) is stored as JSON: it must be `Serialize` and `Deserialize`.
        let infrastructure =
            "map_err(|error| InfrastructureError::from(anyhow::Error::from(error)))?";

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
        "POST" | "PUT" | "PATCH" | "DELETE" => (
            ENDPOINT_COMMAND,
            format!("src/application/commands/{file}.rs"),
        ),
        _ => return Err(format!("{method} is not one of GET|POST|PUT|PATCH|DELETE").into()),
    };

    if !Path::new(&post_it).exists() {
        let generator = if method == "GET" { "query" } else { "command" };

        return Err(format!("{post_it} not found: run cerne g {generator} {name} first").into());
    }

    add_file(
        &format!("src/infrastructure/http/{file}.rs"),
        template,
        context! { name, file, method, path },
    )?;

    insert_lines(
        "src/infrastructure/http/mod.rs",
        "",
        ROUTER_STATE,
        &[&format!(
            ".route({path:?}, axum::routing::{routing}({file}::{file}))"
        )],
        Before,
    )
}

/// `cerne g adapter SmtpNotifier Notifier`: a struct in `infrastructure/` that implements the port `Notifier`.
fn generate_adapter(name: &str, port: &str) -> CliResult {
    if !is_pascal_case(name) || !is_pascal_case(port) {
        return Err(format!(
            "{name} and {port} must be PascalCase names, like SmtpNotifier Notifier"
        )
        .into());
    }

    let port_file = snake_case(port);

    if !Path::new(&format!("src/application/ports/{port_file}.rs")).exists() {
        return Err(
            format!("the port {port} does not exist: run cerne g port {port} first").into(),
        );
    }

    add_file(
        &format!("src/infrastructure/{}.rs", snake_case(name)),
        ADAPTER,
        context! { name, port, port_file },
    )
}

/// In a JSON-RPC project, the arm `"place_order" => methods.command::<PlaceOrderCommand>(..)` (D33).
fn add_rpc_method(file: &str, post_it_use: &str, call: &str) -> CliResult {
    if project()?.http != "jsonrpc" {
        return Ok(());
    }

    let rpc = "src/infrastructure/http/rpc.rs";

    insert_lines(
        rpc,
        post_it_use,
        RPC_LAST_ARM,
        &[&format!(
            "{file:?} => methods.{call}(request.params).await,"
        )],
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

/// Writes the post-it and appends `pub mod <file>;` to the `mod.rs` next to it, so it compiles right away (D12).
fn add_file(path: &str, template: &str, post_it: Value) -> CliResult {
    let file = Path::new(path);
    let mod_rs = file.with_file_name("mod.rs");

    if !mod_rs.exists() {
        return Err(format!(
            "{} not found: run cerne g inside a cerne project",
            mod_rs.display()
        )
        .into());
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
fn insert_lines(
    path: &str,
    uses: &str,
    anchor: &str,
    lines: &[&str],
    position: Position,
) -> CliResult {
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
    // ponytail: best effort, the code is valid without rustfmt, just less tidy
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

/// `qty:i32 id:u64` → `[(qty, i32), (id, u64)]`, in the order given (D13).
fn parse_fields<'a>(
    args: impl Iterator<Item = &'a &'a str>,
) -> Result<Vec<(&'a str, &'a str)>, String> {
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
/// `Order` starts as `Pending`, like the `TransferStatus` of the rde.
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
        return Err(format!(
            "{name}: = only works with enum values, like status=Pending:Pending,Accepted"
        ));
    }

    if kind != "entity" {
        return Err(format!(
            "{name}:{ty}: enum values only work in cerne g entity"
        ));
    }

    let variants: Vec<&str> = ty.split(',').collect();

    if !variants.iter().all(|variant| is_pascal_case(variant)) {
        return Err(format!(
            "{name}:{ty}: every value must be PascalCase, like status:Pending,Accepted"
        ));
    }

    if initial.is_some_and(|initial| !variants.contains(&initial)) {
        return Err(format!(
            "{name}: the initial value must be one of {ty}, like status=Pending:Pending,Accepted"
        ));
    }

    let title = pascal_case(name);
    let section = format!("// --- {title} {}", "-".repeat(80 - 8 - title.len()));

    Ok(context! { name, ty => format!("{owner}{title}"), variants, initial, section })
}

fn is_integer(ty: &str) -> bool {
    matches!(
        ty,
        "i8" | "i16" | "i32" | "i64" | "isize" | "u8" | "u16" | "u32" | "u64" | "usize"
    )
}

/// `value as i64`, unless the value already is an `i64` (clippy refuses a cast to the same type).
fn cast(value: &str, from: &str, to: &str) -> String {
    if from == to {
        value.to_string()
    } else {
        format!("{value} as {to}")
    }
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
