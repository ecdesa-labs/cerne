# Welcome to Cerne

English · [Português](README.pt-BR.md) · [Español](README.es.md)

## Why Cerne?

A system does not become legacy because someone wrote it badly. It becomes legacy because, over the years, it receives thousands of changes that make sense one by one: a union agreement, a new law, a court decision, a tax that changed. Each team solves its ticket as best it can, often in the middle of the night, with an `if` in the middle of the code "just for now". The old rule keeps running next to the new one, because nobody dares to touch it. As the gotoCobol channel puts it:

> "Entropy is born from the pile-up of locally justifiable solutions that are never reorganized as a whole." (translated from Portuguese)

Systems like that are built every day, and the bill arrives years later, when it is time to modernize. Then switching languages is not enough: the business rules are scattered, with no name and no history, and each one has to be dug up before anything can change.

Cerne does not stop the system from changing. It gives every change a proper place and a name:

- **Every rule has a name and an address.** A court decision becomes a `BusinessRule` with its sentence, in the `Business rules` section of the command it affects, not an `if` lost in the middle of the code. Whoever comes later finds the rule by its name, and when it refuses a request, the error says which one.
- **The code has the shape of the board.** Every Event Storming sticky note is a Rust type, and every `execute` has the same sections, in the same order. Different teams, in different years, can write in the same format, because `cerne g` gives all of them the same skeleton.
- **The board and the code tell the same story.** The conversation with the business happens on the board, with the same words as the code. A new rule starts as a sticky note and ends up where the sticky note says.
- **A reaction is a policy, not a side effect.** "Whenever X, do Y" becomes a named `Policy`, and its command goes through the outbox. Nobody has to hunt for where the reaction ended up.

That is what Cerne sets out to do: a system that lasts 30 years and stays readable, not because nothing changed, but because every change stayed in sight.

Inspired by the video [Entropia de software não é a mesma coisa que complexidade estrutural](https://www.youtube.com/watch?v=wTQbXp86m78) (in Portuguese), from the gotoCobol channel.

## What's Cerne?

Cerne is a Rust framework that turns an [Event Storming](https://www.eventstorming.com) board into code: every sticky note becomes a type of yours that implements a Cerne trait, and every flow becomes an `execute` with the sections of the board.

To keep the application in that shape as it grows, Cerne comes with a CLI, `cerne`. `cerne new` creates the project already laid out like the board, and each `cerne g` generates the new piece (a command, an event, an entity, a port) in the right place, with the right sections and already compiling. Whoever joins the project does not have to guess where a new feature lives or how to write it: it is born inside the proposal.

Understanding Event Storming is key to understanding Cerne. The board tells a story with colored sticky notes: an actor sends a command (blue), an aggregate (yellow) changes, a domain event (orange) happens, a policy (lilac) reacts with a new command, external systems (pink) are called, and read models (green) show the result. Cerne splits your application into three layers: Domain, Application and Infrastructure.

### Domain layer

The Domain layer is the heart of the board: entities and aggregates (`Entity`, `Aggregate`), value objects (`ValueObject`), domain events (`DomainEvent`) and the policies that react to them (`Policy`). It is pure and synchronous: no IO happens here. Invariants (`Invariant`) are named rules about what is always true, and when one breaks, the error lists the names as they are written on the board: `["quantity is positive"]`.

### Application layer

The Application layer is where the actors act. A command (`Command`) reads the ports, checks the business rules (`BusinessRule`), changes an aggregate and returns its events, always in that order, so the `execute` reads like a flow of the board. A query (`Query`) returns a read model (`ReadModel`). The ports are async traits for the repositories and the external systems. The commands that policies fire go to an outbox in the same transaction as the aggregate, and the `OutboxPolicyProcessor` runs them, even if the process dies in between.

### Infrastructure layer (optional)

Cerne is, first of all, a framework for modeling the domain and the application, not the infrastructure. The Infrastructure layer is an extra to speed up development: ready-made adapters for the SQL repositories (`cerne::sqlite`, also in memory, and `cerne::postgres`, with the same API and the same SQL) and for HTTP, as REST or JSON-RPC 2.0 (feature `axum`).

`cerne new` only uses them when asked: without `--db`, the project depends on no database adapter, and the outbox lives in memory; `--db`, or `cerne g db` later, adds the database. Nothing in the Domain and Application layers depends on these adapters. The ports are traits, and any adapter that implements them will do: another database, another web framework, a queue. To use Cerne without any of its adapters:

```toml
cerne = { version = "0.1", default-features = false }
```

## Crates

- [`cerne`](https://crates.io/crates/cerne): the library. The features `sqlite` (default), `postgres` and `axum` are the optional Infrastructure layer.
- [`cerne-cli`](https://crates.io/crates/cerne-cli): the `cerne` command, which creates a project laid out like the board (`cerne new`) and generates each sticky note in its place, already compiling (`cerne g`).

## Getting Started

1. Install the `cerne` command (Rust 1.88 or newer):

   ```bash
   cargo install cerne-cli
   ```

2. Create a project with a REST API:

   ```bash
   cerne new shop --http rest
   ```

3. Generate a command and its route, and start the server:

   ```bash
   cd shop
   cerne g command PlaceOrder product:String quantity:u32
   cerne g endpoint PlaceOrder POST /orders
   cargo run
   ```

4. Send the command:

   ```console
   $ curl -X POST localhost:3000/orders -H 'content-type: application/json' -d '{"product": "mug", "quantity": 2}'
   null
   ```

5. Fill in the sticky notes. You may find these resources handy:
   - [The tutorial](docs/en/tutorial.md): a shop, from the board to the code, with every sticky note.
   - [The API documentation](https://docs.rs/cerne)
   - `cerne` with no arguments lists every generator.

## Contributing

Contributions are welcome. See [CONTRIBUTING.md](CONTRIBUTING.md) for how to run the checks and for the code style. Every change that breaks compatibility goes into the [CHANGELOG](CHANGELOG.md).

## License

Cerne is released under the [Apache License 2.0](LICENSE). The code that `cerne new` and `cerne g` generate belongs to whoever generated it, who may use it under any license.
