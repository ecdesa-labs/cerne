//! `cerne`: creates a project laid out like the Event Storming board and generates its post-its.

use minijinja::{Environment, Value, context};
use std::error::Error;
use std::path::Path;
use std::process::ExitCode;
use std::{env, fs, process};

const USAGE: &str = "\
usage:
  cerne new <name>
  cerne g entity <Name> [field:type ...] [field:Value1,Value2 ...] [field=Initial:Value1,Value2 ...] [id:type] [--aggregate]
  cerne g value_object <Name> field:type [field:type ...]
  cerne g event <Name> [field:type ...]
  cerne g command <Name> [field:type ...]
  cerne g read_model <Name> [field:type ...]
  cerne g query <Name> [field:type ...]   (needs the read model <Name>)
  cerne g port <Name>
  cerne g adapter <Name> <Port>";

/// What `cerne new` writes: one folder per layer, each `mod.rs` ready for the generators to append to.
const PROJECT: [(&str, &str); 17] = [
    ("Cargo.toml", include_str!("../templates/Cargo.toml.jinja")),
    (".gitignore", "/target\n"),
    ("rustfmt.toml", include_str!("../templates/rustfmt.toml.jinja")),
    ("src/lib.rs", include_str!("../templates/lib.rs.jinja")),
    ("src/main.rs", MAIN),
    ("src/composition_root.rs", include_str!("../templates/composition_root.rs.jinja")),
    ("src/domain/mod.rs", "pub mod entities;\npub mod events;\npub mod value_objects;\n"),
    ("src/domain/entities/mod.rs", ""),
    ("src/domain/events/mod.rs", ""),
    ("src/domain/value_objects/mod.rs", ""),
    ("src/application/mod.rs", "pub mod commands;\npub mod ports;\npub mod queries;\npub mod read_models;\n"),
    ("src/application/commands/mod.rs", ""),
    ("src/application/ports/mod.rs", ""),
    ("src/application/queries/mod.rs", ""),
    ("src/application/read_models/mod.rs", ""),
    ("src/infrastructure/mod.rs", ""),
    ("tests/board.rs", include_str!("../templates/board.rs.jinja")),
];

const MAIN: &str = include_str!("../templates/main.rs.jinja");
const VALUE_OBJECT: &str = include_str!("../templates/value_object.rs.jinja");
const ENTITY: &str = include_str!("../templates/entity.rs.jinja");
const EVENT: &str = include_str!("../templates/event.rs.jinja");
const COMMAND: &str = include_str!("../templates/command.rs.jinja");
const READ_MODEL: &str = include_str!("../templates/read_model.rs.jinja");
const QUERY: &str = include_str!("../templates/query.rs.jinja");
const PORT: &str = include_str!("../templates/port.rs.jinja");
const ADAPTER: &str = include_str!("../templates/adapter.rs.jinja");

/// The lines `cerne g` adds to files that already exist, right before a line that `cerne new` wrote.
const COMPOSITION_ROOT_FIELD: &str = "pub event_outbox: Box<dyn EventOutbox>,";
const COMPOSITION_ROOT_NEW: &str = "event_outbox: constructor.event_outbox,";
const EVENT_OUTBOX_BUILT: &str = "let event_outbox = ";
const CONSTRUCTOR_BUILT: &str = "= CompositionRootConstructor {";

type CliResult = Result<(), Box<dyn Error>>;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();

    let result = match args.as_slice() {
        ["new", name] => new_project(name),
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

fn new_project(name: &str) -> CliResult {
    let root = Path::new(name);

    if root.exists() {
        return Err(format!("{name} already exists").into());
    }

    let project = context! {
        name,
        crate_name => name.replace('-', "_"),
        cerne_version => env!("CARGO_PKG_VERSION"),
    };

    for (path, template) in PROJECT {
        let file = root.join(path);

        fs::create_dir_all(file.parent().unwrap())?;
        fs::write(&file, render(template, &project)?)?;
    }

    println!("created {name}");

    Ok(())
}

// --- cerne g -----------------------------------------------------------------

/// The crate of the project, from the `name` of its `Cargo.toml`: `my-shop` → `my_shop`.
fn crate_name() -> Result<String, String> {
    let cargo_toml = fs::read_to_string("Cargo.toml")
        .map_err(|_| "Cargo.toml not found: run cerne g inside a cerne project".to_string())?;

    let name = cargo_toml
        .lines()
        .find_map(|line| line.strip_prefix("name = \""))
        .and_then(|rest| rest.strip_suffix('"'))
        .ok_or("Cargo.toml has no name = \"..\"")?;

    Ok(name.replace('-', "_"))
}

fn generate(kind: &str, name: &str, args: &[&str]) -> CliResult {
    if !is_pascal_case(name) {
        return Err(format!("{name} is not a PascalCase name, like Order").into());
    }

    let aggregate = args.contains(&"--aggregate");

    if let Some(flag) = args
        .iter()
        .find(|arg| arg.starts_with("--") && **arg != "--aggregate")
    {
        return Err(format!("unknown flag {flag}\n{USAGE}").into());
    }

    if aggregate && kind != "entity" {
        return Err("--aggregate is only for entity".into());
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
                add_repository_port(name, &file)?;
            }

            Ok(())
        }
        "value_object" if fields.is_empty() => Err("a value object needs at least one field, like value:u64".into()),
        "value_object" => {
            add_file(&format!("src/domain/value_objects/{file}.rs"), VALUE_OBJECT, context! { name, fields, uses })
        }
        "event" => add_file(&format!("src/domain/events/{file}.rs"), EVENT, context! { name, fields, uses }),
        "command" => add_file(&format!("src/application/commands/{file}.rs"), COMMAND, context! { name, fields, uses }),
        "read_model" => {
            add_file(&format!("src/application/read_models/{file}.rs"), READ_MODEL, context! { name, fields, uses })
        }
        "query" if !Path::new(&format!("src/application/read_models/{file}.rs")).exists() => {
            Err(format!("the query {name}Query returns the read model {name}: run cerne g read_model {name} first")
                .into())
        }
        "query" => {
            add_file(&format!("src/application/queries/{file}.rs"), QUERY, context! { name, file, fields, uses })
        }
        "port" => add_file(&format!("src/application/ports/{file}.rs"), PORT, context! { name }),
        _ => Err(format!("unknown generator {kind}\n{USAGE}").into()),
    }
}

/// The port of an aggregate's repository: its field in the `CompositionRoot` and, in `main.rs`, a function where its
/// adapter goes.
fn add_repository_port(name: &str, file: &str) -> CliResult {
    let repository = format!("{file}_repository");
    let repository_type = format!("Box<dyn Repository<{name}>>");

    // --- composition_root.rs: the field ---------------------------------------

    insert_lines(
        "src/composition_root.rs",
        &format!("use crate::domain::entities::{file}::{name};"),
        COMPOSITION_ROOT_FIELD,
        &[&format!("pub {repository}: {repository_type},")],
    )?;

    add_to_use("src/composition_root.rs", "cerne::application", "Repository")?;

    insert_lines(
        "src/composition_root.rs",
        "",
        COMPOSITION_ROOT_NEW,
        &[&format!("{repository}: constructor.{repository},")],
    )?;

    // --- main.rs: the adapter, still to write ---------------------------------

    insert_lines(
        "src/main.rs",
        &format!("use {}::domain::entities::{file}::{name};", crate_name()?),
        EVENT_OUTBOX_BUILT,
        &[&format!("let {repository} = {repository}_adapter();")],
    )?;

    add_to_use("src/main.rs", "cerne::application", "Repository")?;

    // The new field goes right before `event_outbox`, whether the constructor fits in one line or not.
    let main_rs = fs::read_to_string("src/main.rs")?;

    let Some(constructor_start) = main_rs.find(CONSTRUCTOR_BUILT) else {
        return Err(format!("src/main.rs has no line with {CONSTRUCTOR_BUILT:?}: add {repository} by hand").into());
    };
    let constructor_end = constructor_start + main_rs[constructor_start..].find("};").unwrap_or(0);
    let constructor =
        main_rs[constructor_start..constructor_end].replacen("event_outbox", &format!("{repository}, event_outbox"), 1);
    let main_rs = format!("{}{constructor}{}", &main_rs[..constructor_start], &main_rs[constructor_end..]);
    let adapter = format!(
        "\n/// No adapter of Repository<{name}> yet: write one in `infrastructure/` and build it here.\n\
         fn {repository}_adapter() -> {repository_type} {{\n    todo!(\"an adapter of Repository<{name}>\")\n}}\n"
    );

    fs::write("src/main.rs", main_rs + &adapter)?;
    rustfmt(Path::new("src/main.rs"));

    Ok(())
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

/// Adds `lines` right before every line that starts with `anchor`, with its indentation, and the `use` lines at the
/// top. The anchor is a line `cerne new` wrote; nothing else in the file changes.
fn insert_lines(path: &str, uses: &str, anchor: &str, lines: &[&str]) -> CliResult {
    let content = fs::read_to_string(path).map_err(|_| format!("{path} not found"))?;
    let mut file_lines: Vec<String> = content.lines().map(String::from).collect();

    let anchors: Vec<usize> = (0..file_lines.len())
        .filter(|&at| file_lines[at].trim_start().starts_with(anchor))
        .collect();

    if anchors.is_empty() {
        return Err(format!("{path} has no line with {anchor:?}: add {lines:?} by hand").into());
    }

    for &at in anchors.iter().rev() {
        let indentation: String = file_lines[at]
            .chars()
            .take_while(|c| c.is_whitespace())
            .collect();

        for line in lines.iter().rev() {
            file_lines.insert(at, format!("{indentation}{}", line.trim_start()));
        }
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

/// `use cerne::application::{EventOutbox, SyncEventBus};` + `Repository` →
/// `use cerne::application::{EventOutbox, Repository, SyncEventBus};`: one `use` per path, the names in order.
fn add_to_use(path: &str, module: &str, name: &str) -> CliResult {
    let content = fs::read_to_string(path).map_err(|_| format!("{path} not found"))?;
    let path_prefix = format!("use {module}::");

    // `use module::{A, B};` or `use module::A;`, but not `use module::inner::A;`.
    let Some(use_line) = content.lines().find(|line| {
        line.strip_prefix(&path_prefix)
            .is_some_and(|names| !names.contains("::"))
    }) else {
        fs::write(path, format!("use {module}::{name};\n{content}"))?;

        return Ok(());
    };

    let mut names: Vec<&str> = use_line[path_prefix.len()..]
        .trim_end_matches(';')
        .trim_start_matches('{')
        .trim_end_matches('}')
        .split(',')
        .map(str::trim)
        .filter(|existing| !existing.is_empty())
        .collect();

    if names.contains(&name) {
        return Ok(());
    }

    names.push(name);
    names.sort_by_key(|name| name.to_lowercase());

    let merged_use_line = format!("{path_prefix}{{{}}};", names.join(", "));

    fs::write(path, content.replacen(use_line, &merged_use_line, 1))?;

    Ok(())
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
