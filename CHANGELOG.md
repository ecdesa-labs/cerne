# Changelog

Every change that breaks compatibility is listed here. While Cerne is in `0.x`, a minor version (`0.1` → `0.2`) may break it.

## Unreleased

### `cerne`

- `#[entity]` and `#[aggregate]` (in `cerne::domain`, from the new crate `cerne-macros`): on a struct with an `id: Option<<Name>Id>`, they write `impl Entity`, a `<Name>Constructor` with every field but the id, and, for `#[aggregate]`, `impl Aggregate`. A field marked `#[skip_constructor]` stays out of the constructor and starts at its `Default`.
- `#[value_object]`: on a struct of one unnamed field (`OrderId(u64)`), it writes `TryFrom<u64> for OrderId` (through `ValueObject::new`) and `From<OrderId> for u64`, and, if the struct derives `Deserialize` or `Serialize`, `#[serde(try_from = "u64", into = "u64")]`.
- **Breaking:** `validate` left `Entity` for a trait of its own, `Validate`, which `Entity` requires. Move `fn validate` to an `impl Validate for <Name>`, and import `Validate` wherever `.validate()` is called.
- **Breaking:** `Entity::Props` and `ValueObject::Props` are now `Entity::Constructor` and `ValueObject::Constructor`.

### `cerne-cli`

- **Breaking:** `cerne g entity` writes `#[entity]` (or `#[aggregate]`, with `--aggregate`) and an `impl Validate` instead of `impl Entity`, `impl Aggregate` and `<Name>Props`. `Order::new` takes an `OrderConstructor`. An enum field with an initial value (`status=Placed:Placed,Paid`) is `#[skip_constructor]`, and its enum derives `Default` with `#[default]` on the initial value.
- `cerne g value_object` with one field, and the id of `cerne g entity`, write `#[value_object]` instead of `#[serde(try_from, into)]`, `impl TryFrom` and `impl From`.
- **Breaking:** `cerne g value_object` with more than one field writes a `<Name>Constructor` instead of `<Name>Props`.
- `cerne g db` finds the aggregates by their `#[aggregate]`, no longer by `impl Aggregate for`.

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
