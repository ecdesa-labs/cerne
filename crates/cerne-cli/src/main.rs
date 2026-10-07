//! `cerne`: creates a project laid out like the Event Storming board and generates its post-its.

use minijinja::{Environment, Value, context};
use std::error::Error;
use std::path::Path;
use std::process::ExitCode;
use std::{env, fs, process};

const USAGE: &str = "\
usage:
  cerne new <name>
  cerne g entity <Name> [field:type ...] [id:type] [--aggregate]
  cerne g value_object <Name> field:type [field:type ...]
  cerne g event <Name> [field:type ...]
  cerne g command <Name> [field:type ...]";

/// What `cerne new` writes: the layers of D11, each `mod.rs` ready for the generators to append to (D12).
const PROJECT: [(&str, &str); 13] = [
    ("Cargo.toml", include_str!("../templates/Cargo.toml.jinja")),
    (".gitignore", "/target\n"),
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
    ("src/application/mod.rs", "pub mod commands;\n"),
    ("src/application/commands/mod.rs", ""),
    ("src/infrastructure/mod.rs", ""),
    (
        "tests/board.rs",
        include_str!("../templates/board.rs.jinja"),
    ),
];

const VALUE_OBJECT: &str = include_str!("../templates/value_object.rs.jinja");
const ENTITY: &str = include_str!("../templates/entity.rs.jinja");
const EVENT: &str = include_str!("../templates/event.rs.jinja");
const COMMAND: &str = include_str!("../templates/command.rs.jinja");

type CliResult = Result<(), Box<dyn Error>>;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();

    let result = match args.as_slice() {
        ["new", name] => new_project(name),
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

    let project = context! { name, crate_name => name.replace('-', "_") };

    for (path, template) in PROJECT {
        let file = root.join(path);

        fs::create_dir_all(file.parent().unwrap())?;
        fs::write(&file, render(template, &project)?)?;
    }

    println!("created {name}");

    Ok(())
}

// --- cerne g -----------------------------------------------------------------

fn generate(kind: &str, name: &str, args: &[&str]) -> CliResult {
    let name_is_pascal_case = name.starts_with(|c: char| c.is_ascii_uppercase())
        && name.chars().all(|c| c.is_ascii_alphanumeric());

    if !name_is_pascal_case {
        return Err(format!("{name} is not a PascalCase name, like Order").into());
    }

    let aggregate = args.contains(&"--aggregate");

    if aggregate && kind != "entity" {
        return Err("--aggregate is only for entity".into());
    }

    let mut fields = parse_fields(args.iter().filter(|arg| **arg != "--aggregate"))?;
    let file = snake_case(name);

    match kind {
        "entity" => {
            let id_type = take_id_type(&mut fields).unwrap_or("u64".into());
            let id_field = context! { name => "value", ty => id_type };

            add_file(
                &format!("src/domain/value_objects/{file}_id.rs"),
                VALUE_OBJECT,
                context! { name => format!("{name}Id"), fields => vec![id_field] },
            )?;

            add_file(
                &format!("src/domain/entities/{file}.rs"),
                ENTITY,
                context! { name, file, fields, aggregate },
            )
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
        "command" => add_file(
            &format!("src/application/commands/{file}.rs"),
            COMMAND,
            context! { name, fields },
        ),
        _ => Err(format!("unknown generator {kind}\n{USAGE}").into()),
    }
}

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

    // ponytail: best effort, the code is valid without rustfmt, just less tidy
    let _ = process::Command::new("rustfmt")
        .args(["--edition", "2024"])
        .arg(file)
        .status();

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

// --- Helpers -----------------------------------------------------------------

fn render(template: &str, data: &Value) -> Result<String, minijinja::Error> {
    let mut environment = Environment::new();

    environment.set_trim_blocks(true);
    environment.set_lstrip_blocks(true);
    environment.set_keep_trailing_newline(true);

    environment.render_str(template, data)
}

/// `qty:i32 id:u64` → `[{name: qty, ty: i32}, {name: id, ty: u64}]`, in the order given (D13).
fn parse_fields<'a>(args: impl Iterator<Item = &'a &'a str>) -> Result<Vec<Value>, String> {
    args.map(|arg| match arg.split_once(':') {
        Some((name, ty)) if !name.is_empty() && !ty.is_empty() => Ok(context! { name, ty }),
        _ => Err(format!("{arg} is not a field: use name:type, like qty:i32")),
    })
    .collect()
}

/// Removes the `id:<type>` field, if there is one, and gives back its type: the id becomes a value object.
fn take_id_type(fields: &mut Vec<Value>) -> Option<String> {
    let position = fields.iter().position(|field| {
        field
            .get_attr("name")
            .is_ok_and(|name| name.as_str() == Some("id"))
    })?;

    let id_field = fields.remove(position);

    Some(id_field.get_attr("ty").ok()?.to_string())
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
