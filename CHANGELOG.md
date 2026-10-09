# Changelog

Every change that breaks compatibility is listed here. While Cerne is in `0.x`, a minor version (`0.1` → `0.2`) may break it.

## 0.3.0

The event buses are no longer Actix actors: they are plain structs on Tokio, so an application runs on `#[tokio::main]` (Axum, Actix Web or none) instead of `#[actix::main]`. The command opens its transaction on the database and passes it to every repository: the `CompositionRoot` no longer builds itself again per transaction.

### `cerne`

- **Breaking:** `PublishEvents` is gone, and so are `.start()`, `.send(..)` and `.do_send(..)`. Build the bus with `SyncEventBus::new(composition_root, on_error)` and publish with `sync_event_bus.publish(execution.events).await`: it still runs one chain at a time, in the order the calls arrived, and returns when the chain ends. `async_event_bus.publish(execution.events)` is not `async`: it starts every event in its own Tokio task and returns at once. On the multi-thread runtime, the tasks run on every core.
- **Breaking:** `publish` returns nothing: the errors go to `on_error`, as before. `on_error` must now be `Send + Sync`.
- **Breaking:** `cerne` depends on `tokio` (`rt` and `sync`) instead of `actix`. Run `main` on `#[tokio::main]` and the tests on `#[tokio::test]`.
- **Breaking:** the transaction is an argument. `Repository<A>` is now `Repository<A, Transaction>`, and `load`, `save` and `EventOutbox::store` take `&mut Transaction` first: `composition_root.order_repository.save(&mut transaction, order).await?`. `Transaction` is the application's own type. The `CompositionRoot` no longer has `begin` and `commit`: the command opens the transaction on the database (`let mut transaction = composition_root.database.begin().await?;`) and commits it (`transaction.commit().await?`). The repositories and the event outbox are built once, in `main`, and no new `CompositionRoot` is built per transaction.

### `cerne-cli`

- **Breaking:** `cerne new` writes `#[tokio::main]` in `main.rs`, and the project depends on `tokio` (`macros` and `rt-multi-thread`) instead of `actix`.
- **Breaking:** `cerne new` writes `src/infrastructure/database.rs`, with a `Database` (`begin`) and a `Transaction` (`commit`) to write on your database, and the `CompositionRoot` gets a `database` field instead of `begin` and `commit`. `cerne g command` opens the transaction with `composition_root.database.begin()`, and `cerne g entity --aggregate` writes `Box<dyn Repository<Order, Transaction>>`.

## 0.2.0

Cerne no longer brings infrastructure: the library keeps the ports, and the adapters are the application's. The outbox stores events instead of commands, and an event bus (an Actix actor) runs the policies.

### `cerne`

- **Breaking:** the modules `cerne::sqlite`, `cerne::postgres` and `cerne::http`, and the features `sqlite`, `postgres` and `axum`, are gone, with `SqliteDatabase`, `SqliteOutbox`, `PostgresDatabase`, `PostgresOutbox` and `InMemoryOutbox`. Write an adapter of `Repository` and of `EventOutbox` on the database of the application, and serve HTTP with the web framework of your choice. Drop `default-features = false` and `features = [..]` from the `cerne` dependency.
- **Breaking:** `Outbox` is now `EventOutbox`, with `store` only: `next_pending`, `mark_done`, `mark_failed` and `send_events` are gone. It stores events, not commands: `OutboxEntry` has the fields `event` (`OrderPlaced` → `"order_placed"`) and `json`, and `OutboxEntry::new(&order_placed)?` builds it from any event that derives `Serialize`. Each command stores its events itself, in its transaction: `transaction.event_outbox.store(OutboxEntry::new(&order_placed)?).await?`.
- **Breaking:** `TransactionalPorts` and `execute_in_transaction` are gone. `begin` and `commit` are methods of the application's `CompositionRoot`, and the command calls them: `let transaction = composition_root.begin().await?;` as its first section, `transaction.commit().await?` after storing its events. Whoever calls the command publishes the events it returns.
- **Breaking:** `PolicyProcessor`, `InlinePolicyProcessor`, `TokioPolicyProcessor`, `OutboxPolicyProcessor`, `CommandRegistry`, `StoredCommand` and `CommandRun` are gone. The policies run on an event bus, an Actix actor: `SyncEventBus::new(composition_root, on_error).start()` runs one chain at a time, and `AsyncEventBus` starts every event as soon as it arrives. Publish with `sync_event_bus.send(PublishEvents(execution.events)).await`. A failing command goes to `on_error`, and nothing runs again: how each policy survives a failure is up to the application. Run `main` on `#[actix::main]` and the tests on `#[actix::test]`.
- **Breaking:** the command of a policy no longer needs `Serialize` or `Deserialize`, and `FiredPolicy` is only a name and a command: `FiredPolicy::outbox_entry` is gone.
- `command_name` is gone.
- **Breaking:** the empty trait `ReadModel` is gone: `Query::ReadModel` only needs `Send`. Delete every `impl ReadModel for ..` and its `use`.
- `#[entity]` and `#[aggregate]` (in `cerne::domain`, from the new crate `cerne-macros`): on a struct with an `id: Option<<Name>Id>`, they write `impl Entity`, a `<Name>Constructor` with every field but the id, and, for `#[aggregate]`, `impl Aggregate`. A field marked `#[skip_constructor]` stays out of the constructor and starts at its `Default`.
- `#[value_object]`: on a struct of one unnamed field (`OrderId(u64)`), it writes `TryFrom<u64> for OrderId` (through `ValueObject::new`) and `From<OrderId> for u64`, and, if the struct derives `Deserialize` or `Serialize`, `#[serde(try_from = "u64", into = "u64")]`.
- `invariant!("quantity is positive", quantity_is_positive)` and `business_rule!(..)`: an `Invariant` or a `BusinessRule` from a name and a condition, without writing the `move ||` closure.
- `policy!("whenever an order is placed, charge the customer", true, ChargeOrderCommand { order_id, total })`: a `Policy` from a name, a condition and the command, without writing the closures. The command is only built if the policy fires, and it takes the values it uses: no `.clone()` inside.
- **Breaking:** `Policies::new(vec![..]).trigger()` is now `Policies::trigger([..])`, and the `then` of `Policy::new` is a `FnOnce`.
- **Breaking:** `Invariants::new(vec![..]).enforce()` is now `Invariants::enforce([..])`, and `BusinessRules::new(vec![..]).check()` is now `BusinessRules::check([..])`. Both take any `IntoIterator`, an array or a `Vec`.
- **Breaking:** `validate` left `Entity` for a trait of its own, `Validate`, which `Entity` requires. Move `fn validate` to an `impl Validate for <Name>`, and import `Validate` wherever `.validate()` is called.
- **Breaking:** `Entity::Props` and `ValueObject::Props` are now `Entity::Constructor` and `ValueObject::Constructor`.
- **Breaking:** the type parameter `Ports` of `Command`, `Query` and `DomainEvent` is named `CompositionRoot`.

### `cerne-cli`

- **Breaking:** `cerne new` takes no flags: `--db` and `--http` are gone, and so are `cerne g db`, `cerne g http`, `cerne g endpoint` and `cerne g command --policy`. A flag the CLI does not know is an error. The project has no `[package.metadata.cerne]`, no `sqlx`, no `axum` and no `tokio`, and depends on `actix`.
- **Breaking:** the generated `CompositionRoot` has an `event_outbox: Box<dyn EventOutbox>` and `begin`/`commit` methods with a `todo!()`, and `main.rs` builds every adapter still missing in a function with a `todo!()` (`event_outbox_adapter()`, `order_repository_adapter()`), and a `SyncEventBus`. `cerne g entity --aggregate` adds the port of the repository (`order_repository: Box<dyn Repository<Order>>`) and its function in `main.rs`, no longer a SQL repository and its migration.
- **Breaking:** `cerne g command` writes a command without `Serialize`/`Deserialize`, with the section `Transaction` first and the `commit` at the end of `Domain events`. `cerne g event` writes an event that derives `Serialize` and `Deserialize`. `cerne g query` writes a query without `Deserialize`.
- **Breaking:** `cerne g read_model` no longer writes `impl ReadModel`.
- **Breaking:** `cerne g entity` writes `#[entity]` (or `#[aggregate]`, with `--aggregate`) and an `impl Validate` instead of `impl Entity`, `impl Aggregate` and `<Name>Props`. `Order::new` takes an `OrderConstructor`. An enum field with an initial value (`status=Placed:Placed,Paid`) is `#[skip_constructor]`, and its enum derives `Default` with `#[default]` on the initial value.
- `cerne g value_object` with one field, and the id of `cerne g entity`, write `#[value_object]` instead of `#[serde(try_from, into)]`, `impl TryFrom` and `impl From`.
- **Breaking:** `cerne g value_object` with more than one field writes a `<Name>Constructor` instead of `<Name>Props`.
- `cerne g entity` and `cerne g value_object` write `Invariants::enforce([])?`, and `cerne g event` writes `Policies::trigger([])`.
- `cerne new` writes a `rustfmt.toml`: lines of up to 120 columns, with the arguments of a call on one line while they fit, one item per line in a list past 80 columns, and struct literals and method chains broken as before.
- **Breaking:** the field that `cerne g entity --aggregate` adds to the `CompositionRoot` for the repository of an aggregate is `<name>_repository`, no longer the table name: `order_repository` instead of `orders`. Rename the field and every `ports.orders` in the commands and queries.
- **Breaking:** what `cerne new` wrote as the struct `Ports` in `src/ports.rs` is the struct `CompositionRoot` in `src/composition_root.rs`, and the generated code names it `composition_root`: `impl Command<CompositionRoot>`, `composition_root: &CompositionRoot`, `composition_root.order_repository`. Rename the file, the module, the struct and the variables.
- **Breaking:** `cerne new` writes a `CompositionRootConstructor` next to the `CompositionRoot`, with every adapter already built, and `CompositionRoot::new` takes it and only keeps them: `CompositionRoot::new(composition_root_constructor)` instead of `Ports::new(database)`. `src/main.rs` builds every adapter. In an existing project, move what `new` built into `main.rs` and `begin`, and build the constructor in a variable wherever `CompositionRoot::new` is called.

## 0.1.0

The first release.

### `cerne`

- The sticky notes of the board as traits: `Command<Ports>` (returns `Executed { output, events }`), `Entity`, `Aggregate`, `ValueObject`, `DomainEvent<Ports>`, `Query<Ports>` and `ReadModel`.
- Named rules: `Invariant` + `Invariants` and `BusinessRule` + `BusinessRules`. A failure is a `DomainError::Violations` with the names.
- Policies: `Policy` + `Policies`. Their commands go to an outbox in the same transaction as the aggregate (`Outbox`, `TransactionalPorts`, `CommandRegistry`), and the `OutboxPolicyProcessor` runs them. Also `InlinePolicyProcessor` and `TokioPolicyProcessor`.
- `InMemoryOutbox`: the outbox of a project without a database.
- Errors in three categories, gathered in `cerne::Error`: `DomainError`, `ApplicationError` and `InfrastructureError`.
- SQL adapters with the same API: `cerne::sqlite` (default feature, also in memory) and `cerne::postgres` (feature `postgres`).
- HTTP (feature `axum`): `cerne::Error` as a REST answer, and JSON-RPC 2.0 with `params` by name or by position.

### `cerne-cli`

- `cerne new <name> [--db memory|sqlite|postgres] [--http rest|jsonrpc]`: a project laid out like the board, one folder per layer. Without `--db`, it depends on no database adapter and the outbox lives in memory (`InMemoryOutbox`); `cerne g db` adds the database later, with the SQL repository of every aggregate that already exists.
- `cerne g entity | value_object | event | command | read_model | query | endpoint | http | db | port | adapter`: each sticky note in its place, compiling right away. `--aggregate` also generates the SQL repository and its migration; `--policy` registers the command in the outbox.
