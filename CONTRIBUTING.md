# Contributing to Cerne

Before a pull request, all three must pass:

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

`cargo test` also builds the tutorial: it runs the `cerne` commands of `docs/en/tutorial.md`, checks every block marked `<!-- generated: <path> -->` against what the CLI wrote, writes every block marked `<!-- file: <path> -->` and runs `cargo clippy` and `cargo test` on the project. A change to the tutorial goes into its three translations (`docs/en`, `docs/pt-BR`, `docs/es`): only the prose, the diagrams and the comments of the commands are translated, and a test checks that the code is the same in all three.

## Code style

The goal: whoever reads the code knows which part of the Event Storming board they are in. When in doubt, the version with fewer concepts wins.

- **A concept of the user is a struct of its own plus a trait of the library:** `ValueObject`, `Entity`, `Aggregate`, `DomainEvent<CompositionRoot>`, `Command<CompositionRoot>`, `Query<CompositionRoot>`. Never a generic struct with a `name` and a `payload`.
- **Named collections:** `Invariant`/`Invariants`, `BusinessRule`/`BusinessRules` and `Policy`/`Policies` follow the same shape: each item has a `name: &'static str` and a closure, and the collection runs them all and returns names. Invariants and business rules are written with `invariant!` and `business_rule!` and run with `Invariants::enforce([..])?` and `BusinessRules::check([..])?`. Policies are written with `policy!("name", condition, Command { .. })`, always three arguments (`true` for one that always fires), and fired with `Policies::trigger([..])`.
- **The condition goes into a variable before,** named like a sentence: `let quantity_is_positive = self.quantity > 0;`, then `invariant!("quantity is positive", quantity_is_positive)`.
- **Every event goes into a variable before the return:** `let order_placed = OrderPlaced { .. };`, then `Ok(Executed { output, events: vec![Box::new(order_placed)] })`.
- **Two arguments of the same type are variables named like the parameters,** so swapping them shows when reading.
- **The result of an `execute` goes into a `*_execution` variable;** what the caller needs comes in the `Output`, never by downcasting events.
- **Formatting:** `rustfmt.toml` allows 120 columns and keeps the arguments of a call on one line while they fit. A list breaks one item per line past 80 columns (two invariants do, one does not), and struct literals and method chains break as in rustfmt's default.
- **Sections with an 80-column divider:** the body of an `execute` follows the board, in this order: `Transaction`, `Domain service`, `Ports`, `Business rules`, `External system: <Name>`, `Aggregate`, `Domain events` (store each event with `OutboxEntry::new`, then `commit`). The same goes for aggregates, `trigger_policies` (one variable per policy, ending in `_policy`), `main` (one block per actor) and tests (one block per flow).
- **The variable names the event bus:** `sync_event_bus`, `async_event_bus`. Never just `bus`.
- **No infrastructure in the library or the CLI:** Cerne brings ports (`Repository`, `EventOutbox`), never adapters. An adapter missing from the generated `main` is a `fn <port>_adapter()` with a `todo!()`.
- **Whatever is a repository ends in `_repository`:** the field of the `CompositionRoot` is `order_repository: Box<dyn Repository<Order>>`, and a command reads `composition_root.order_repository.load(..)`. Never just `orders`.
- **The `CompositionRoot` has a constructor, like an entity, and nothing is built in `new`:** `CompositionRoot::new` takes a `CompositionRootConstructor` with every adapter already built and only keeps them. `main` builds them; `begin`, which the command calls, builds a new `CompositionRoot` with the repositories and the event outbox on its transaction and hands over the external systems with `Arc::clone`. The order is: build each adapter into a variable, with no `Box` or `Arc`; run the migration, if there is one; build the constructor into a variable, which is the only place each adapter is wrapped (`order_repository: Box::new(order_repository)`, `catalog: Arc::new(catalog)`); then `CompositionRoot::new(composition_root_constructor)`.
- **Method names say the part of the flow:** `trigger_policies`, `begin`, `commit`, `PublishEvents`.
- **A create command receives no id:** the repository decides it on insert.
- **Errors:** the domain returns `EnforcementResult<T>`; commands, queries and ports return `Result<T, cerne::Error>`.
- **A synchronous domain, an asynchronous application:** `Command`, `Query` and the ports use `#[async_trait]`, and the event buses are Actix actors; entities, events, invariants, rules and policies are never async.
- **Code, tests and messages in English.**
