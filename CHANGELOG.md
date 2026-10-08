# Changelog

Every change that breaks compatibility is listed here. While Cerne is in `0.x`, a minor version (`0.1` → `0.2`) may break it.

## 0.1.0

The first release.

### `cerne`

- The sticky notes of the board as traits: `Command<Ports>` (returns `Executed { output, events }`), `Entity`, `Aggregate`, `ValueObject`, `DomainEvent<Ports>`, `Query<Ports>` and `ReadModel`.
- Named rules: `Invariant` + `Invariants` and `BusinessRule` + `BusinessRules`. A failure is a `DomainError::Violations` with the names.
- Policies: `Policy` + `Policies`. Their commands go to an outbox in the same transaction as the aggregate (`Outbox`, `TransactionalPorts`, `CommandRegistry`), and the `OutboxPolicyProcessor` runs them. Also `InlinePolicyProcessor` and `TokioPolicyProcessor`.
- Errors in three categories, gathered in `cerne::Error`: `DomainError`, `ApplicationError` and `InfrastructureError`.
- SQL adapters with the same API: `cerne::sqlite` (default feature, also in memory) and `cerne::postgres` (feature `postgres`).
- HTTP (feature `axum`): `cerne::Error` as a REST answer, and JSON-RPC 2.0 with `params` by name or by position.

### `cerne-cli`

- `cerne new <name> [--db memory|sqlite|postgres] [--http rest|jsonrpc]`: a project laid out like the board, one folder per layer.
- `cerne g entity | value_object | event | command | read_model | query | endpoint | http | port | adapter`: each sticky note in its place, compiling right away. `--aggregate` also generates the SQL repository and its migration; `--policy` registers the command in the outbox.
