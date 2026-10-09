# Contributing to Cerne

Before a pull request, all three must pass:

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

`cargo test` also builds the tutorial: it runs the `cerne` commands of `docs/en/tutorial.md`, checks every block marked `<!-- generated: <path> -->` against what the CLI wrote, writes every block marked `<!-- file: <path> -->` and runs `cargo clippy` and `cargo test` on the project. A change to the tutorial goes into its three translations (`docs/en`, `docs/pt-BR`, `docs/es`): only the prose, the diagrams and the comments of the commands are translated, and a test checks that the code is the same in all three.

The Postgres tests need a Postgres. With one running:

```bash
DATABASE_URL=postgres://postgres:cerne@localhost:5432/postgres cargo test -p cerne --no-default-features --features postgres --test postgres
```

## Code style

The goal: whoever reads the code knows which part of the Event Storming board they are in. When in doubt, the version with fewer concepts wins.

- **A concept of the user is a struct of its own plus a trait of the library:** `ValueObject`, `Entity`, `Aggregate`, `DomainEvent<Ports>`, `Command<Ports>`, `Query<Ports>`. Never a generic struct with a `name` and a `payload`.
- **Named collections:** `Invariant`/`Invariants`, `BusinessRule`/`BusinessRules` and `Policy`/`Policies` follow the same shape: each item has a `name: &'static str` and a closure, and the collection runs them all and returns names. Invariants and business rules are written with `invariant!` and `business_rule!` and run with `Invariants::enforce([..])?` and `BusinessRules::check([..])?`. Policies are written with `policy!("name", condition, Command { .. })`, always three arguments (`true` for one that always fires), and fired with `Policies::trigger([..])`.
- **The condition goes into a variable before,** named like a sentence: `let quantity_is_positive = self.quantity > 0;`, then `invariant!("quantity is positive", quantity_is_positive)`.
- **Every event goes into a variable before the return:** `let order_placed = OrderPlaced { .. };`, then `Ok(Executed { output, events: vec![Box::new(order_placed)] })`.
- **Two arguments of the same type are variables named like the parameters,** so swapping them shows when reading.
- **The result of an `execute` goes into a `*_execution` variable;** what the caller needs comes in the `Output`, never by downcasting events.
- **Formatting:** `rustfmt.toml` allows 120 columns and keeps the arguments of a call on one line while they fit. A list breaks one item per line past 80 columns (two invariants do, one does not), and struct literals and method chains break as in rustfmt's default.
- **Sections with an 80-column divider:** the body of an `execute` follows the board, in this order: `Domain service`, `Ports`, `Business rules`, `External system: <Name>`, `Aggregate`, `Domain events`. The same goes for aggregates, `trigger_policies` (one variable per policy, ending in `_policy`), `main` (one block per actor) and tests (one block per flow).
- **The variable names the processor:** `outbox_policy_processor`, `sync_policy_processor`, `async_policy_processor`. Never just `processor`.
- **A repository in memory is SQLite in memory** (`SqliteDatabase::in_memory()`), with the production adapter. Never a `Vec` or a `HashMap`.
- **Whatever is a repository ends in `_repository`:** the field of the `Ports` is `order_repository: Box<dyn Repository<Order>>`, and a command reads `ports.order_repository.load(..)`. Never just `orders`.
- **Method names say the part of the flow:** `send_events`, `send_command`, `trigger_policies`.
- **A create command receives no id:** the repository decides it on insert.
- **Errors:** the domain returns `EnforcementResult<T>`; commands, repositories and processors return `Result<T, cerne::Error>`.
- **A synchronous domain, an asynchronous application:** `Command`, `Repository` and `PolicyProcessor` use `#[async_trait]`; entities, events, invariants, rules and policies are never async.
- **Code, tests and messages in English.**
