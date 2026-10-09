# Tutorial: a shop, from the board to the code

## Sticky note → code

| Sticky note | Concept | In Cerne |
|---|---|---|
| 🟦 | Command | trait `Command<CompositionRoot>`, which returns `Executed { output, events }` |
| 🟨 | Aggregate / Entity | attributes `#[entity]` and `#[aggregate]`, which write the traits `Entity` and `Aggregate`; the invariants go in the trait `Validate` |
| — | Value Object | trait `ValueObject`, also the type of every entity id; attribute `#[value_object]`, for a value object of one value |
| 🟧 | Domain Event | trait `DomainEvent<CompositionRoot>`; the command stores it in the `EventOutbox` |
| 🟪 | Policy | `Policy` + `Policies`; the `SyncEventBus` or the `AsyncEventBus` runs them |
| 🩷 | External System | a port (an async trait of the application) and its adapters, gathered in the `CompositionRoot` |
| 🟩 | Read Model / Query | trait `Query<CompositionRoot>`, which returns a read model: a struct of plain fields |
| — | Invariants | `Invariant` + `Invariants`: what is always true about an entity or a value object |
| — | Business rules | `BusinessRule` + `BusinessRules`: what must hold for a command to run |

## Install

```bash
cargo install cerne-cli
```

Cerne needs Rust 1.88 or newer.

## The board

The board has two flows. The customer places an order; the shop checks the stock, and a policy charges the customer. Then the customer reads the order.

```mermaid
flowchart LR
  customer["👤 Customer"]:::actor --> place["Place order<br/>PlaceOrderCommand"]:::command
  place --> rules["Stock covers the quantity"]:::rule
  catalog["Catalog"]:::external -.-> rules
  rules --> order["Order::new() → Placed"]:::aggregate
  order --> placed["Order placed<br/>OrderPlaced"]:::event
  placed --> policy["Whenever an order is placed,<br/>charge the customer"]:::policy
  policy --> charge["Charge order<br/>ChargeOrderCommand"]:::command
  charge --> payments["Payments"]:::external
  payments --> paid_order["order.pay() → Paid"]:::aggregate
  paid_order --> paid["Order paid<br/>OrderPaid"]:::event
  customer2["👤 Customer"]:::actor --> summary["Order summary<br/>OrderSummaryQuery"]:::read_model
  classDef actor fill:#ffe46b,stroke:#c9a800,color:#221f1a
  classDef command fill:#8cc6f5,stroke:#3d8fd1,color:#221f1a
  classDef rule fill:#97dccf,stroke:#3fa892,color:#221f1a
  classDef aggregate fill:#fff3b3,stroke:#c9b84a,color:#221f1a
  classDef event fill:#ffb25c,stroke:#d97a14,color:#221f1a
  classDef policy fill:#c9b4f2,stroke:#7d5cc4,color:#221f1a
  classDef external fill:#f8adc9,stroke:#d0578a,color:#221f1a
  classDef read_model fill:#a8e6a1,stroke:#4caf50,color:#221f1a
```

Every block of code below is a whole file of the project. A test of this repository runs these commands, writes these files and runs `cargo test` on the result, so the tutorial always compiles.

## 1. The project and its sticky notes

Creates the `shop` project.

```bash
cerne new shop
```

Enters the project: the `cerne g` commands run inside it.

```bash
cd shop
```

Creates the 🟨 aggregate `Order`: an order, with the port of its repository and a status that starts at `Placed`. The `id` field the CLI creates by itself: a value object `OrderId`, with a `u64` inside. With `id:<type>` (for example, `id:String`), `OrderId` holds a `String` instead of the `u64`.

```bash
cerne g entity Order product:String quantity:u32 total:u64 status=Placed:Placed,Paid --aggregate
```

Creates the 🟧 event `OrderPlaced`: the order was placed.

```bash
cerne g event OrderPlaced order_id:OrderId total:u64
```

Creates the 🟧 event `OrderPaid`: the order was paid.

```bash
cerne g event OrderPaid order_id:OrderId
```

Creates the 🟦 command `PlaceOrder`: the customer places an order.

```bash
cerne g command PlaceOrder product:String quantity:u32
```

Creates the 🟦 command `ChargeOrder`: charge the order. A policy sends it, not an actor; for the CLI, it is a command like any other.

```bash
cerne g command ChargeOrder order_id:OrderId total:u64
```

Creates the 🩷 port `Catalog`: the external system with the prices and the stock.

```bash
cerne g port Catalog
```

Creates `InMemoryCatalog`: a catalog in memory, for the tests and for this tutorial.

```bash
cerne g adapter InMemoryCatalog Catalog
```

Creates the 🩷 port `Payments`: the external system that charges the customer.

```bash
cerne g port Payments
```

Creates `InMemoryPayments`: a payment system in memory, for the tests and for this tutorial.

```bash
cerne g adapter InMemoryPayments Payments
```

Creates the 🟩 read model `OrderSummary`: the summary of the order that the customer sees.

```bash
cerne g read_model OrderSummary product:String quantity:u32 total:u64 status:String
```

Creates the query `OrderSummary`: it finds that summary by the id of the order.

```bash
cerne g query OrderSummary order_id:OrderId
```

The fields are `name:type`, and `status=Placed:Placed,Paid` creates an enum with the values `Placed` and `Paid` that starts at `Placed`. After each command the project still compiles. `--aggregate` also adds the field `order_repository` to the `CompositionRoot`, and, in `main.rs`, a function `order_repository_adapter()` with a `todo!()`: Cerne brings no adapter, and the repository is yours to write.

```console
$ tree shop
shop
├── Cargo.toml
├── rustfmt.toml
├── src
│   ├── application
│   │   ├── commands
│   │   │   ├── charge_order.rs
│   │   │   ├── mod.rs
│   │   │   └── place_order.rs
│   │   ├── mod.rs
│   │   ├── ports
│   │   │   ├── catalog.rs
│   │   │   ├── mod.rs
│   │   │   └── payments.rs
│   │   ├── queries
│   │   │   ├── mod.rs
│   │   │   └── order_summary.rs
│   │   └── read_models
│   │       ├── mod.rs
│   │       └── order_summary.rs
│   ├── composition_root.rs
│   ├── domain
│   │   ├── entities
│   │   │   ├── mod.rs
│   │   │   └── order.rs
│   │   ├── events
│   │   │   ├── mod.rs
│   │   │   ├── order_paid.rs
│   │   │   └── order_placed.rs
│   │   ├── mod.rs
│   │   └── value_objects
│   │       ├── mod.rs
│   │       └── order_id.rs
│   ├── infrastructure
│   │   ├── in_memory_catalog.rs
│   │   ├── in_memory_payments.rs
│   │   └── mod.rs
│   ├── lib.rs
│   └── main.rs
└── tests
    └── board.rs
```

Each layer has its folder. `domain/` is pure and synchronous: no IO. `application/` is asynchronous: commands, queries and the ports they use. `infrastructure/` holds the adapters, which you write: here, all of them in memory.

What is left is to fill in the sticky notes.

## 2. Value object: `OrderId`

A value object has no identity: two `OrderId(7)` are the same thing. It only exists if its invariants hold, and it never changes. The id of every entity is a value object, so a `0` never becomes an order id, not even when read back from JSON.

The `src/domain/value_objects/order_id.rs` as `cerne g entity Order product:String quantity:u32 total:u64 status=Placed:Placed,Paid --aggregate` generated it:

<!-- generated: src/domain/value_objects/order_id.rs -->
```rust
use cerne::domain::{EnforcementResult, Invariants, ValueObject, value_object};
use serde::{Deserialize, Serialize};

/// In JSON it is the `u64` itself, and reading it back goes through `new`: the invariants hold there too.
#[value_object]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrderId(u64);

impl ValueObject for OrderId {
    type Constructor = u64;

    fn new(value: u64) -> EnforcementResult<Self> {
        Invariants::enforce([])?;

        Ok(Self(value))
    }
}
```

Filled in:

<!-- file: src/domain/value_objects/order_id.rs -->
```rust
use cerne::domain::{EnforcementResult, Invariants, ValueObject, invariant, value_object};
use serde::{Deserialize, Serialize};

/// In JSON it is the `u64` itself, and reading it back goes through `new`: the invariants hold there too.
#[value_object]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrderId(u64);

impl ValueObject for OrderId {
    type Constructor = u64;

    fn new(value: u64) -> EnforcementResult<Self> {
        let order_id_is_positive = value > 0;

        Invariants::enforce([invariant!("order id is positive", order_id_is_positive)])?;

        Ok(Self(value))
    }
}
```

Each invariant is a name plus a condition, written with `invariant!`. The condition goes into a variable before, named like the sentence on the board. `Invariants::enforce` runs all of them and, if any fails, returns `DomainError::Violations` with every failing name.

`#[value_object]` writes what a value object of one value has in common: `TryFrom<u64> for OrderId`, which goes through `new`, `From<OrderId> for u64`, which gives the `u64` back, and, because `OrderId` derives `Serialize` and `Deserialize`, `#[serde(try_from = "u64", into = "u64")]`. `new`, with the invariants, is yours. The repository of step 4 uses the `TryFrom` to make the id of a new order.

## 3. Aggregate: `Order` 🟨

The `src/domain/entities/order.rs` as `cerne g entity Order product:String quantity:u32 total:u64 status=Placed:Placed,Paid --aggregate` generated it:

<!-- generated: src/domain/entities/order.rs -->
```rust
use crate::domain::value_objects::order_id::OrderId;
use cerne::domain::{EnforcementResult, Invariants, Validate, aggregate};

// --- Status ------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum OrderStatus {
    #[default]
    Placed,
    Paid,
}

// --- Aggregate ---------------------------------------------------------------

#[aggregate]
#[derive(Debug, Clone, PartialEq)]
pub struct Order {
    pub id: Option<OrderId>,
    pub product: String,
    pub quantity: u32,
    pub total: u64,
    #[skip_constructor]
    pub status: OrderStatus,
}

// --- Invariants --------------------------------------------------------------

impl Validate for Order {
    fn validate(self) -> EnforcementResult<Self> {
        Invariants::enforce([])?;

        Ok(self)
    }
}

// --- State transitions -------------------------------------------------------

impl Order {}
```

Filled in:

<!-- file: src/domain/entities/order.rs -->
```rust
use crate::domain::value_objects::order_id::OrderId;
use cerne::domain::{EnforcementResult, Invariants, Validate, aggregate, invariant};

// --- Status ------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum OrderStatus {
    #[default]
    Placed,
    Paid,
}

// --- Aggregate ---------------------------------------------------------------

#[aggregate]
#[derive(Debug, Clone, PartialEq)]
pub struct Order {
    pub id: Option<OrderId>,
    pub product: String,
    pub quantity: u32,
    pub total: u64,
    #[skip_constructor]
    pub status: OrderStatus,
}

// --- Invariants --------------------------------------------------------------

impl Validate for Order {
    fn validate(self) -> EnforcementResult<Self> {
        let order_has_a_product = !self.product.is_empty();
        let quantity_is_positive = self.quantity > 0;

        Invariants::enforce([
            invariant!("order has a product", order_has_a_product),
            invariant!("quantity is positive", quantity_is_positive),
        ])?;

        Ok(self)
    }
}

// --- State transitions -------------------------------------------------------

impl Order {
    pub fn pay(self) -> EnforcementResult<Self> {
        Self {
            status: OrderStatus::Paid,
            ..self
        }
        .validate()
    }
}
```

- `#[aggregate]` writes what every aggregate has in common: `impl Entity` (the `id()`, the `with_id` the repository calls and `new`), `impl Aggregate` and the `OrderConstructor`. An entity that is not an aggregate (`cerne g entity` without `--aggregate`) gets `#[entity]`, which writes the same except `impl Aggregate`: no repository takes it.
- `Order::new` receives the `OrderConstructor`, with every field but two: the id, which the repository decides on the first `save` (until then, `id()` is `None`), and `status`, marked `#[skip_constructor]`. A skipped field starts at its `Default`: `OrderStatus` derives `Default`, with `#[default]` on `Placed`, the initial value of `status=Placed:Placed,Paid`.
- `validate`, in `impl Validate`, holds the invariants: the CLI generates it empty, for you to fill in. It runs in `new` and in every state transition (`pay`): an order that breaks an invariant never exists in memory.
- The state transitions are methods that consume the order and return the next one.

## 4. External systems: ports and adapters 🩷

A port is an async trait of the application, and every method returns `Result<_, cerne::Error>`. The command does not know which adapter is behind it.

The `src/application/ports/catalog.rs` as `cerne g port Catalog` generated it:

<!-- generated: src/application/ports/catalog.rs -->
```rust
use cerne::async_trait;

/// External system "Catalog": one async method per thing the commands ask of it, each returning
/// `Result<_, cerne::Error>`.
#[async_trait]
pub trait Catalog: Send + Sync {}
```

Filled in:

<!-- file: src/application/ports/catalog.rs -->
```rust
use cerne::{Error, async_trait};

/// External system "Catalog": what the shop sells, at what price, and how much is left.
#[async_trait]
pub trait Catalog: Send + Sync {
    /// The price of one unit, in cents.
    async fn unit_price(&self, product: &str) -> Result<u64, Error>;

    async fn units_in_stock(&self, product: &str) -> Result<u32, Error>;
}
```

The `src/application/ports/payments.rs` as `cerne g port Payments` generated it:

<!-- generated: src/application/ports/payments.rs -->
```rust
use cerne::async_trait;

/// External system "Payments": one async method per thing the commands ask of it, each returning
/// `Result<_, cerne::Error>`.
#[async_trait]
pub trait Payments: Send + Sync {}
```

Filled in:

<!-- file: src/application/ports/payments.rs -->
```rust
use crate::domain::value_objects::order_id::OrderId;
use cerne::{Error, async_trait};

/// External system "Payments": charges the customer. The order id is the idempotency key: charging the same order
/// twice charges it once.
#[async_trait]
pub trait Payments: Send + Sync {
    async fn charge(&self, order_id: &OrderId, amount: u64) -> Result<(), Error>;
}
```

The in-memory adapters stand in for the real systems in the tests and in this tutorial:

The `src/infrastructure/in_memory_catalog.rs` as `cerne g adapter InMemoryCatalog Catalog` generated it:

<!-- generated: src/infrastructure/in_memory_catalog.rs -->
```rust
use crate::application::ports::catalog::Catalog;
use cerne::async_trait;

/// An adapter of the port `Catalog`.
pub struct InMemoryCatalog;

#[async_trait]
impl Catalog for InMemoryCatalog {}
```

Filled in:

<!-- file: src/infrastructure/in_memory_catalog.rs -->
```rust
use crate::application::ports::catalog::Catalog;
use cerne::{ApplicationError, Error, async_trait};

/// A catalog with fixed products: (name, unit price in cents, units in stock).
pub struct InMemoryCatalog {
    pub products: Vec<(&'static str, u64, u32)>,
}

impl InMemoryCatalog {
    fn product(&self, product: &str) -> Result<(&'static str, u64, u32), Error> {
        let found = self.products.iter().find(|(name, _, _)| *name == product);

        Ok(*found.ok_or(ApplicationError::NotFound("product"))?)
    }
}

#[async_trait]
impl Catalog for InMemoryCatalog {
    async fn unit_price(&self, product: &str) -> Result<u64, Error> {
        let (_, unit_price, _) = self.product(product)?;

        Ok(unit_price)
    }

    async fn units_in_stock(&self, product: &str) -> Result<u32, Error> {
        let (_, _, units_in_stock) = self.product(product)?;

        Ok(units_in_stock)
    }
}
```

The `src/infrastructure/in_memory_payments.rs` as `cerne g adapter InMemoryPayments Payments` generated it:

<!-- generated: src/infrastructure/in_memory_payments.rs -->
```rust
use crate::application::ports::payments::Payments;
use cerne::async_trait;

/// An adapter of the port `Payments`.
pub struct InMemoryPayments;

#[async_trait]
impl Payments for InMemoryPayments {}
```

Filled in:

<!-- file: src/infrastructure/in_memory_payments.rs -->
```rust
use crate::application::ports::payments::Payments;
use crate::domain::value_objects::order_id::OrderId;
use cerne::{Error, async_trait};
use std::sync::Mutex;

/// Keeps every charge instead of calling a payment provider.
#[derive(Default)]
pub struct InMemoryPayments {
    pub charges: Mutex<Vec<(OrderId, u64)>>,
}

#[async_trait]
impl Payments for InMemoryPayments {
    async fn charge(&self, order_id: &OrderId, amount: u64) -> Result<(), Error> {
        let mut charges = self.charges.lock().unwrap();

        if !charges.iter().any(|(charged, _)| charged == order_id) {
            charges.push((order_id.clone(), amount));
        }

        Ok(())
    }
}
```

Two ports are traits of Cerne itself: `Repository<Order>`, which loads and saves the order, and `EventOutbox`, which stores every event a command produces. `cerne g adapter` only writes adapters of the ports of the project, so these two are written by hand. In a real application they sit on the database, and `begin` opens a transaction in it; here, an `InMemoryDatabase` holds the orders and the events, and every clone of it shares the same data.

<!-- file: src/infrastructure/in_memory_database.rs -->
```rust
use crate::domain::entities::order::Order;
use crate::domain::value_objects::order_id::OrderId;
use cerne::application::{EventOutbox, OutboxEntry, Repository};
use cerne::domain::Entity;
use cerne::{ApplicationError, Error, async_trait};
use std::sync::{Arc, Mutex};

/// What a database would hold, in memory: cloning it shares the same data. There is no transaction: a command that
/// fails halfway keeps what it already wrote.
#[derive(Clone, Default)]
pub struct InMemoryDatabase {
    pub orders: Arc<Mutex<Vec<Order>>>,
    pub event_outbox: Arc<Mutex<Vec<OutboxEntry>>>,
}

// --- Repository<Order> -------------------------------------------------------

pub struct InMemoryOrderRepository {
    database: InMemoryDatabase,
}

impl InMemoryOrderRepository {
    pub fn new(database: InMemoryDatabase) -> Self {
        Self { database }
    }
}

#[async_trait]
impl Repository<Order> for InMemoryOrderRepository {
    async fn load(&self, order_id: &OrderId) -> Result<Order, Error> {
        let orders = self.database.orders.lock().unwrap();
        let order = orders.iter().find(|order| order.id() == Some(order_id));

        Ok(order.cloned().ok_or(ApplicationError::NotFound("order"))?)
    }

    async fn save(&self, order: Order) -> Result<OrderId, Error> {
        let mut orders = self.database.orders.lock().unwrap();

        let order_id = match order.id() {
            Some(order_id) => order_id.clone(),
            None => OrderId::try_from(orders.len() as u64 + 1)?,
        };

        orders.retain(|saved_order| saved_order.id() != Some(&order_id));
        orders.push(order.with_id(order_id.clone()));

        Ok(order_id)
    }
}

// --- EventOutbox -------------------------------------------------------------

pub struct InMemoryEventOutbox {
    database: InMemoryDatabase,
}

impl InMemoryEventOutbox {
    pub fn new(database: InMemoryDatabase) -> Self {
        Self { database }
    }
}

#[async_trait]
impl EventOutbox for InMemoryEventOutbox {
    async fn store(&self, outbox_entry: OutboxEntry) -> Result<(), Error> {
        self.database.event_outbox.lock().unwrap().push(outbox_entry);

        Ok(())
    }
}
```

The `src/infrastructure/mod.rs` as `cerne g adapter InMemoryCatalog Catalog` and `cerne g adapter InMemoryPayments Payments` generated it:

<!-- generated: src/infrastructure/mod.rs -->
```rust
pub mod in_memory_catalog;
pub mod in_memory_payments;
```

Filled in:

<!-- file: src/infrastructure/mod.rs -->
```rust
pub mod in_memory_catalog;
pub mod in_memory_database;
pub mod in_memory_payments;
```

The `CompositionRoot` gathers every port, and `CompositionRoot::new` only keeps the adapters it gets: `main.rs` builds them. `cerne g entity --aggregate` already added the `order_repository`; the database and the two external systems are added by hand.

`begin` and `commit` are yours too: `cerne new` writes them with a `todo!()`, because only the adapters know how to open a transaction. `begin` builds a new `CompositionRoot` whose repository and event outbox write in the transaction. The external systems stay the same, `Arc::clone` of the same adapters, because a call to them cannot be rolled back. In memory there is no transaction: `begin` builds the adapters again on the same `InMemoryDatabase`, and `commit` has nothing to do.

The `src/composition_root.rs` as `cerne new shop` and `cerne g entity Order product:String quantity:u32 total:u64 status=Placed:Placed,Paid --aggregate` generated it:

<!-- generated: src/composition_root.rs -->
```rust
use crate::domain::entities::order::Order;
use cerne::Error;
use cerne::application::{EventOutbox, Repository};

/// The composition root: every port the commands and queries can use.
pub struct CompositionRoot {
    pub order_repository: Box<dyn Repository<Order>>,
    pub event_outbox: Box<dyn EventOutbox>,
}

/// What `CompositionRoot::new` takes: every adapter, already built.
pub struct CompositionRootConstructor {
    pub order_repository: Box<dyn Repository<Order>>,
    pub event_outbox: Box<dyn EventOutbox>,
}

impl CompositionRoot {
    pub fn new(constructor: CompositionRootConstructor) -> Self {
        Self {
            order_repository: constructor.order_repository,
            event_outbox: constructor.event_outbox,
        }
    }

    /// A composition root whose repositories and event outbox write in one new transaction. External systems are not
    /// part of it: the new composition root gets the same adapters.
    pub async fn begin(&self) -> Result<CompositionRoot, Error> {
        todo!("open a transaction on your adapters and build a CompositionRoot on it")
    }

    /// Makes every write of the transaction permanent; dropping it without `commit` rolls them back.
    pub async fn commit(self) -> Result<(), Error> {
        todo!("commit the transaction of your adapters")
    }
}
```

Filled in:

<!-- file: src/composition_root.rs -->
```rust
use crate::application::ports::catalog::Catalog;
use crate::application::ports::payments::Payments;
use crate::domain::entities::order::Order;
use crate::infrastructure::in_memory_database::{InMemoryDatabase, InMemoryEventOutbox, InMemoryOrderRepository};
use cerne::Error;
use cerne::application::{EventOutbox, Repository};
use std::sync::Arc;

/// The composition root: every port the commands and queries can use.
pub struct CompositionRoot {
    pub database: InMemoryDatabase,
    pub order_repository: Box<dyn Repository<Order>>,
    pub event_outbox: Box<dyn EventOutbox>,
    pub catalog: Arc<dyn Catalog>,
    pub payments: Arc<dyn Payments>,
}

/// What `CompositionRoot::new` takes: every adapter, already built.
pub struct CompositionRootConstructor {
    pub database: InMemoryDatabase,
    pub order_repository: Box<dyn Repository<Order>>,
    pub event_outbox: Box<dyn EventOutbox>,
    pub catalog: Arc<dyn Catalog>,
    pub payments: Arc<dyn Payments>,
}

impl CompositionRoot {
    pub fn new(constructor: CompositionRootConstructor) -> Self {
        Self {
            database: constructor.database,
            order_repository: constructor.order_repository,
            event_outbox: constructor.event_outbox,
            catalog: constructor.catalog,
            payments: constructor.payments,
        }
    }

    /// In memory there is no transaction: the repository and the event outbox are built again on the same data.
    pub async fn begin(&self) -> Result<CompositionRoot, Error> {
        let transaction = self.database.clone();

        let order_repository = InMemoryOrderRepository::new(transaction.clone());
        let event_outbox = InMemoryEventOutbox::new(transaction.clone());

        let composition_root_constructor = CompositionRootConstructor {
            database: transaction,
            order_repository: Box::new(order_repository),
            event_outbox: Box::new(event_outbox),
            catalog: Arc::clone(&self.catalog),
            payments: Arc::clone(&self.payments),
        };

        Ok(CompositionRoot::new(composition_root_constructor))
    }

    /// Nothing to make permanent: every write is already in memory.
    pub async fn commit(self) -> Result<(), Error> {
        Ok(())
    }
}
```

## 5. Command: `PlaceOrderCommand` 🟦

The command is a struct with what the actor sends, and nothing else: no id, since the repository decides it. The `execute` follows the board from left to right, one section per sticky note:

The `src/application/commands/place_order.rs` as `cerne g command PlaceOrder product:String quantity:u32` generated it:

<!-- generated: src/application/commands/place_order.rs -->
```rust
use crate::composition_root::CompositionRoot;
use cerne::application::{Command, Executed};
use cerne::{Error, async_trait};

/// Who sends it: an actor, or a policy?
pub struct PlaceOrderCommand {
    pub product: String,
    pub quantity: u32,
}

#[async_trait]
impl Command<CompositionRoot> for PlaceOrderCommand {
    type Output = ();

    async fn execute(&self, composition_root: &CompositionRoot) -> Result<Executed<(), CompositionRoot>, Error> {
        // --- Transaction -----------------------------------------------------

        let transaction = composition_root.begin().await?;

        // --- Ports -----------------------------------------------------------

        // --- Business rules --------------------------------------------------

        // --- Aggregate -------------------------------------------------------

        // --- Domain events ---------------------------------------------------

        transaction.commit().await?;

        Ok(Executed {
            output: (),
            events: vec![],
        })
    }
}
```

Filled in:

<!-- file: src/application/commands/place_order.rs -->
```rust
use crate::composition_root::CompositionRoot;
use crate::domain::entities::order::{Order, OrderConstructor};
use crate::domain::events::order_placed::OrderPlaced;
use crate::domain::value_objects::order_id::OrderId;
use cerne::application::{Command, Executed, OutboxEntry};
use cerne::domain::{BusinessRules, Entity, business_rule};
use cerne::{Error, async_trait};

/// Actor: the customer.
pub struct PlaceOrderCommand {
    pub product: String,
    pub quantity: u32,
}

#[async_trait]
impl Command<CompositionRoot> for PlaceOrderCommand {
    type Output = OrderId; // the id of the new order, for the customer to follow it

    async fn execute(&self, composition_root: &CompositionRoot) -> Result<Executed<OrderId, CompositionRoot>, Error> {
        // --- Transaction -----------------------------------------------------

        let transaction = composition_root.begin().await?;

        // --- Ports -----------------------------------------------------------

        let unit_price = transaction.catalog.unit_price(&self.product).await?;
        let units_in_stock = transaction.catalog.units_in_stock(&self.product).await?;

        // --- Business rules --------------------------------------------------

        let stock_covers_the_quantity = units_in_stock >= self.quantity;

        BusinessRules::check([business_rule!("stock covers the quantity", stock_covers_the_quantity)])?;

        // --- Aggregate -------------------------------------------------------

        let total = unit_price * u64::from(self.quantity);

        let order = Order::new(OrderConstructor {
            product: self.product.clone(),
            quantity: self.quantity,
            total,
        })?;

        let order_id = transaction.order_repository.save(order).await?;

        // --- Domain events ---------------------------------------------------

        let order_placed = OrderPlaced {
            order_id: order_id.clone(),
            total,
        };

        transaction.event_outbox.store(OutboxEntry::new(&order_placed)?).await?;
        transaction.commit().await?;

        Ok(Executed {
            output: order_id,
            events: vec![Box::new(order_placed)],
        })
    }
}
```

- **Transaction:** `composition_root.begin()` opens it, and every port below is read from `transaction`.
- **Ports:** every read the command needs, before any decision.
- **Business rules:** each condition in a variable, then `BusinessRules::check([business_rule!(..)])?`. A business rule needs the outside world (here, the stock); an invariant only needs the entity itself.
- **Aggregate:** the change, then `save`, which returns the id.
- **Domain events:** each event in a variable, stored in the event outbox with `OutboxEntry::new`, in the same transaction as the order: both are saved, or neither is. Then the `commit`, and `Ok(Executed { output, events })`. The `output` goes back to whoever sent the command (here, the id of the new order), and so do the `events`, which that caller publishes on the event bus (step 6).

## 6. Domain event and policy: `OrderPlaced` 🟧 🟪

An event says what happened, in the past tense. It derives `Serialize` so the command can store it in the event outbox, and `Deserialize` to read it back from there. Its `trigger_policies` lists the policies that react to it: each one is a `policy!` with three arguments: a name, a condition (`true` for a policy that always fires) and the command it fires. The command is only built if the condition holds, and it takes the values it uses: here `order_id` and `total`, read from the event before. `Policies::trigger` returns the policies that fired.

The `src/domain/events/order_placed.rs` as `cerne g event OrderPlaced order_id:OrderId total:u64` generated it:

<!-- generated: src/domain/events/order_placed.rs -->
```rust
use crate::composition_root::CompositionRoot;
use crate::domain::value_objects::order_id::OrderId;
use cerne::domain::{DomainEvent, EnforcementResult, FiredPolicy, Policies};
use serde::{Deserialize, Serialize};

/// `Serialize`: the command that produces it stores it in the event outbox; `Deserialize`, to read it back from there.
#[derive(Serialize, Deserialize)]
pub struct OrderPlaced {
    pub order_id: OrderId,
    pub total: u64,
}

impl DomainEvent<CompositionRoot> for OrderPlaced {
    fn trigger_policies(&self) -> EnforcementResult<Vec<FiredPolicy<CompositionRoot>>> {
        // --- Policies --------------------------------------------------------

        Ok(Policies::trigger([]))
    }
}
```

Filled in:

<!-- file: src/domain/events/order_placed.rs -->
```rust
use crate::application::commands::charge_order::ChargeOrderCommand;
use crate::composition_root::CompositionRoot;
use crate::domain::value_objects::order_id::OrderId;
use cerne::domain::{DomainEvent, EnforcementResult, FiredPolicy, Policies, policy};
use serde::{Deserialize, Serialize};

/// `Serialize`: the command that produces it stores it in the event outbox; `Deserialize`, to read it back from there.
#[derive(Serialize, Deserialize)]
pub struct OrderPlaced {
    pub order_id: OrderId,
    pub total: u64,
}

impl DomainEvent<CompositionRoot> for OrderPlaced {
    fn trigger_policies(&self) -> EnforcementResult<Vec<FiredPolicy<CompositionRoot>>> {
        // --- Policies --------------------------------------------------------

        let order_id = self.order_id.clone();
        let total = self.total;

        let charge_the_customer_policy =
            policy!("whenever an order is placed, charge the customer", true, ChargeOrderCommand { order_id, total });

        Ok(Policies::trigger([charge_the_customer_policy]))
    }
}
```

The policy does not run the command: the event bus does. Whoever sent `PlaceOrderCommand` publishes its events with `sync_event_bus.send(PublishEvents(events))`, and, for each event, the bus calls `trigger_policies`, executes the command of each policy that fired and publishes the events that command returns, until the chain ends. The bus opens no transaction: each command opens its own.

| | `SyncEventBus` | `AsyncEventBus` |
|---|---|---|
| Order | one chain at a time: the next `PublishEvents` waits for the current chain to end | every event starts as soon as it arrives |
| `send(..).await` | returns when the whole chain has run | returns as soon as the events have started |

Both are Actix actors, so `main` runs on `#[actix::main]` and the tests on `#[actix::test]`.

The bus hopes for the best. A command that fails, or an event whose invariants fail, goes to the `on_error` the bus was built with, and the bus moves on: nothing runs again, and nothing marks the event. How each policy survives a failure is up to you. Take a policy that sends an e-mail through a notification API: if the API is down, the command fails, and the e-mail is lost, unless you do something about it. You can read the `event_outbox` table again and publish what was left behind, or let the command try again itself; then the API may get the same e-mail twice, unless it takes an idempotency key. Cerne writes every event to the outbox; reading it back is yours.

`OrderPaid` stays as `cerne g event` generated it: no policy reacts to it yet.

## 7. The command a policy fires: `ChargeOrderCommand` 🟦

The `src/application/commands/charge_order.rs` as `cerne g command ChargeOrder order_id:OrderId total:u64` generated it:

<!-- generated: src/application/commands/charge_order.rs -->
```rust
use crate::composition_root::CompositionRoot;
use crate::domain::value_objects::order_id::OrderId;
use cerne::application::{Command, Executed};
use cerne::{Error, async_trait};

/// Who sends it: an actor, or a policy?
pub struct ChargeOrderCommand {
    pub order_id: OrderId,
    pub total: u64,
}

#[async_trait]
impl Command<CompositionRoot> for ChargeOrderCommand {
    type Output = ();

    async fn execute(&self, composition_root: &CompositionRoot) -> Result<Executed<(), CompositionRoot>, Error> {
        // --- Transaction -----------------------------------------------------

        let transaction = composition_root.begin().await?;

        // --- Ports -----------------------------------------------------------

        // --- Business rules --------------------------------------------------

        // --- Aggregate -------------------------------------------------------

        // --- Domain events ---------------------------------------------------

        transaction.commit().await?;

        Ok(Executed {
            output: (),
            events: vec![],
        })
    }
}
```

Filled in:

<!-- file: src/application/commands/charge_order.rs -->
```rust
use crate::composition_root::CompositionRoot;
use crate::domain::entities::order::OrderStatus;
use crate::domain::events::order_paid::OrderPaid;
use crate::domain::value_objects::order_id::OrderId;
use cerne::application::{Command, Executed, OutboxEntry};
use cerne::domain::{BusinessRules, business_rule};
use cerne::{Error, async_trait};

/// No actor: the policy "whenever an order is placed, charge the customer" fires it.
pub struct ChargeOrderCommand {
    pub order_id: OrderId,
    pub total: u64,
}

#[async_trait]
impl Command<CompositionRoot> for ChargeOrderCommand {
    type Output = ();

    async fn execute(&self, composition_root: &CompositionRoot) -> Result<Executed<(), CompositionRoot>, Error> {
        // --- Transaction -----------------------------------------------------

        let transaction = composition_root.begin().await?;

        // --- Ports -----------------------------------------------------------

        let order = transaction.order_repository.load(&self.order_id).await?;

        // --- Business rules --------------------------------------------------

        let order_is_still_placed = order.status == OrderStatus::Placed;

        BusinessRules::check([business_rule!("order is still placed", order_is_still_placed)])?;

        // --- External system: Payments ---------------------------------------

        transaction.payments.charge(&self.order_id, self.total).await?;

        // --- Aggregate -------------------------------------------------------

        let paid_order = order.pay()?;

        transaction.order_repository.save(paid_order).await?;

        // --- Domain events ---------------------------------------------------

        let order_paid = OrderPaid {
            order_id: self.order_id.clone(),
        };

        transaction.event_outbox.store(OutboxEntry::new(&order_paid)?).await?;
        transaction.commit().await?;

        Ok(Executed {
            output: (),
            events: vec![Box::new(order_paid)],
        })
    }
}
```

If the charge fails, the error goes to the `on_error` of the bus, and the order stays `Placed`. Charging it again is up to the application, and it is safe here: the order id is the idempotency key of the payment, and the rule "order is still placed" refuses an order that was already paid. The section `External system: Payments` sits between the rules and the aggregate.

## 8. Query and read model: `OrderSummary` 🟩

The read model is what the actor sees on the screen: plain fields, no behavior. `cerne g read_model` generated it, and it stays as it is. The query reads the ports and builds it; it changes nothing, so it opens no transaction:

The `src/application/queries/order_summary.rs` as `cerne g query OrderSummary order_id:OrderId` generated it:

<!-- generated: src/application/queries/order_summary.rs -->
```rust
use crate::application::read_models::order_summary::OrderSummary;
use crate::composition_root::CompositionRoot;
use crate::domain::value_objects::order_id::OrderId;
use cerne::application::Query;
use cerne::{Error, async_trait};

pub struct OrderSummaryQuery {
    pub order_id: OrderId,
}

#[async_trait]
impl Query<CompositionRoot> for OrderSummaryQuery {
    type ReadModel = OrderSummary;

    async fn execute(&self, _ports: &CompositionRoot) -> Result<OrderSummary, Error> {
        // --- Ports -----------------------------------------------------------

        // --- Read model ------------------------------------------------------

        todo!("read the ports and build the OrderSummary read model")
    }
}
```

Filled in:

<!-- file: src/application/queries/order_summary.rs -->
```rust
use crate::application::read_models::order_summary::OrderSummary;
use crate::composition_root::CompositionRoot;
use crate::domain::value_objects::order_id::OrderId;
use cerne::application::Query;
use cerne::{Error, async_trait};

pub struct OrderSummaryQuery {
    pub order_id: OrderId,
}

#[async_trait]
impl Query<CompositionRoot> for OrderSummaryQuery {
    type ReadModel = OrderSummary;

    async fn execute(&self, composition_root: &CompositionRoot) -> Result<OrderSummary, Error> {
        // --- Ports -----------------------------------------------------------

        let order = composition_root
            .order_repository
            .load(&self.order_id)
            .await?;

        // --- Read model ------------------------------------------------------

        let order_summary = OrderSummary {
            product: order.product,
            quantity: order.quantity,
            total: order.total,
            status: format!("{:?}", order.status),
        };

        Ok(order_summary)
    }
}
```

## 9. The board as a test

`tests/board.rs` has one block per flow, on the same in-memory adapters as `main.rs`.

The `tests/board.rs` as `cerne new shop` generated it:

<!-- generated: tests/board.rs -->
```rust
//! One block per flow of the board: an actor sends a command, and the test checks its events and the policies that fired.
```

Filled in:

<!-- file: tests/board.rs -->
```rust
//! One block per flow of the board: an actor sends a command, and the test checks its events and the policies that fired.

use actix::Actor;
use cerne::Error;
use cerne::application::{Command, PublishEvents, Query, SyncEventBus};
use cerne::domain::{DomainError, ValueObject};
use shop::application::commands::place_order::PlaceOrderCommand;
use shop::application::queries::order_summary::OrderSummaryQuery;
use shop::application::read_models::order_summary::OrderSummary;
use shop::composition_root::{CompositionRoot, CompositionRootConstructor};
use shop::domain::value_objects::order_id::OrderId;
use shop::infrastructure::in_memory_catalog::InMemoryCatalog;
use shop::infrastructure::in_memory_database::{InMemoryDatabase, InMemoryEventOutbox, InMemoryOrderRepository};
use shop::infrastructure::in_memory_payments::InMemoryPayments;
use std::sync::Arc;

fn composition_root(payments: Arc<InMemoryPayments>) -> Arc<CompositionRoot> {
    let database = InMemoryDatabase::default();

    let order_repository = InMemoryOrderRepository::new(database.clone());
    let event_outbox = InMemoryEventOutbox::new(database.clone());

    let catalog = InMemoryCatalog {
        products: vec![("mug", 3000, 10)],
    };

    let composition_root_constructor = CompositionRootConstructor {
        database,
        order_repository: Box::new(order_repository),
        event_outbox: Box::new(event_outbox),
        catalog: Arc::new(catalog),
        payments,
    };

    Arc::new(CompositionRoot::new(composition_root_constructor))
}

#[actix::test]
async fn the_customer_places_an_order_and_the_policy_charges_it() -> anyhow::Result<()> {
    let payments = Arc::new(InMemoryPayments::default());
    let composition_root = composition_root(Arc::clone(&payments));

    let sync_event_bus = SyncEventBus::new(Arc::clone(&composition_root), |error| eprintln!("policy: {error}")).start();

    // --- Customer: places an order -------------------------------------------

    let place_order = PlaceOrderCommand {
        product: "mug".into(),
        quantity: 2,
    };

    let place_order_execution = place_order.execute(&composition_root).await?;

    let order_id = place_order_execution.output;

    sync_event_bus.send(PublishEvents(place_order_execution.events)).await?;

    // --- Policy: whenever an order is placed, charge the customer -----------

    assert_eq!(*payments.charges.lock().unwrap(), vec![(order_id.clone(), 6000)]);

    let stored_events: Vec<String> = composition_root
        .database
        .event_outbox
        .lock()
        .unwrap()
        .iter()
        .map(|outbox_entry| outbox_entry.event.clone())
        .collect();

    assert_eq!(stored_events, ["order_placed", "order_paid"]);

    // --- Customer: reads the order -------------------------------------------

    let order_summary_query = OrderSummaryQuery { order_id };

    let order_summary = order_summary_query.execute(&composition_root).await?;

    let paid_order_summary = OrderSummary {
        product: "mug".into(),
        quantity: 2,
        total: 6000,
        status: "Paid".into(),
    };

    assert_eq!(order_summary, paid_order_summary);

    Ok(())
}

#[actix::test]
async fn the_domain_refuses_what_breaks_a_rule_or_an_invariant() -> anyhow::Result<()> {
    let composition_root = composition_root(Arc::default());

    // --- Business rule: stock covers the quantity ----------------------------

    let too_many_mugs = PlaceOrderCommand {
        product: "mug".into(),
        quantity: 11,
    };

    let refused = too_many_mugs.execute(&composition_root).await;

    assert!(matches!(
        refused,
        Err(Error::Domain(DomainError::Violations(violations))) if violations == ["stock covers the quantity"]
    ));

    // --- Invariant: quantity is positive -------------------------------------

    let no_mugs = PlaceOrderCommand {
        product: "mug".into(),
        quantity: 0,
    };

    let refused = no_mugs.execute(&composition_root).await;

    assert!(matches!(
        refused,
        Err(Error::Domain(DomainError::Violations(violations))) if violations == ["quantity is positive"]
    ));

    // --- Value object: order id is positive ----------------------------------

    assert!(OrderId::new(0).is_err());

    Ok(())
}
```

The test does what an actor does: it executes the command, keeps the `output` and publishes the `events` on the `SyncEventBus`. `send(..).await` returns when the whole chain has run, so the charge is already there on the next line.

```bash
cargo test
```

## 10. The application: `main.rs`

`main.rs` builds every adapter, the composition root and the event bus, then has one block per actor. `cerne new` writes a function with a `todo!()` for each adapter still missing; filled in, they give way to the in-memory adapters.

The `src/main.rs` as `cerne new shop` and `cerne g entity Order product:String quantity:u32 total:u64 status=Placed:Placed,Paid --aggregate` generated it:

<!-- generated: src/main.rs -->
```rust
use actix::Actor;
use cerne::application::{EventOutbox, Repository, SyncEventBus};
use shop::composition_root::{CompositionRoot, CompositionRootConstructor};
use shop::domain::entities::order::Order;
use std::sync::Arc;

#[actix::main]
async fn main() -> anyhow::Result<()> {
    // --- Composition root ----------------------------------------------------

    let order_repository = order_repository_adapter();
    let event_outbox = event_outbox_adapter();

    let composition_root_constructor = CompositionRootConstructor {
        order_repository,
        event_outbox,
    };

    let composition_root = Arc::new(CompositionRoot::new(composition_root_constructor));

    // --- Event bus: the policies of every event ------------------------------

    #[expect(unused_variables, reason = "the blocks of the actors publish their events on it")]
    let sync_event_bus = SyncEventBus::new(composition_root, |error| eprintln!("policy: {error}")).start();

    // One block per actor: execute the command, then publish its events with
    // `sync_event_bus.send(PublishEvents(execution.events)).await?`.

    Ok(())
}

/// No adapter of EventOutbox yet: write one in `infrastructure/` and build it here.
fn event_outbox_adapter() -> Box<dyn EventOutbox> {
    todo!("an adapter of EventOutbox")
}

/// No adapter of Repository<Order> yet: write one in `infrastructure/` and build it here.
fn order_repository_adapter() -> Box<dyn Repository<Order>> {
    todo!("an adapter of Repository<Order>")
}
```

Filled in:

<!-- file: src/main.rs -->
```rust
use actix::Actor;
use cerne::application::{Command, PublishEvents, Query, SyncEventBus};
use shop::application::commands::place_order::PlaceOrderCommand;
use shop::application::queries::order_summary::OrderSummaryQuery;
use shop::composition_root::{CompositionRoot, CompositionRootConstructor};
use shop::infrastructure::in_memory_catalog::InMemoryCatalog;
use shop::infrastructure::in_memory_database::{InMemoryDatabase, InMemoryEventOutbox, InMemoryOrderRepository};
use shop::infrastructure::in_memory_payments::InMemoryPayments;
use std::sync::Arc;

#[actix::main]
async fn main() -> anyhow::Result<()> {
    // --- Composition root ----------------------------------------------------

    let database = InMemoryDatabase::default();

    let order_repository = InMemoryOrderRepository::new(database.clone());
    let event_outbox = InMemoryEventOutbox::new(database.clone());

    let catalog = InMemoryCatalog {
        products: vec![("mug", 3000, 10), ("t-shirt", 5000, 3)],
    };
    let payments = InMemoryPayments::default();

    let composition_root_constructor = CompositionRootConstructor {
        database,
        order_repository: Box::new(order_repository),
        event_outbox: Box::new(event_outbox),
        catalog: Arc::new(catalog),
        payments: Arc::new(payments),
    };

    let composition_root = Arc::new(CompositionRoot::new(composition_root_constructor));

    // --- Event bus: the policies of every event ------------------------------

    let sync_event_bus = SyncEventBus::new(Arc::clone(&composition_root), |error| eprintln!("policy: {error}")).start();

    // --- Customer: places an order -------------------------------------------

    let place_order = PlaceOrderCommand {
        product: "mug".into(),
        quantity: 2,
    };

    let place_order_execution = place_order.execute(&composition_root).await?;

    let order_id = place_order_execution.output;

    sync_event_bus.send(PublishEvents(place_order_execution.events)).await?;

    // --- Customer: reads the order -------------------------------------------

    let order_summary_query = OrderSummaryQuery { order_id };

    let order_summary = order_summary_query.execute(&composition_root).await?;

    println!("{order_summary:?}");

    Ok(())
}
```

```bash
cargo run
```

```console
$ cargo run
OrderSummary { product: "mug", quantity: 2, total: 6000, status: "Paid" }
```

The order is already `Paid`: the `SyncEventBus` ran the `ChargeOrderCommand` before `send(..).await` returned. A web server, a queue consumer or a CLI would take the place of these blocks: each one executes the command and publishes its events, the same way.

## 11. When something goes wrong

Every error is a `cerne::Error`, in one of three categories. An adapter of HTTP (or of anything else) matches on the category to decide its answer:

| Error | When |
|---|---|
| `DomainError::Violations` | an invariant or a business rule failed; it carries every failing name, as written on the board |
| `ApplicationError::NotFound` | the repository (or an adapter) found nothing |
| `InfrastructureError` | database, network, queue |

In the test of step 9, `PlaceOrderCommand { product: "mug".into(), quantity: 11 }` comes back as `DomainError::Violations(["stock covers the quantity"])`.

## Other options

- **A real database:** write an adapter of `Repository<Order>` and of `EventOutbox` on it (with `sqlx`, `diesel` or any other), and a `begin` that opens a transaction and builds them on it. The `event_outbox` table is yours: reading it back to publish what was left behind is how a policy survives a crash.
- **HTTP, a queue, a CLI:** each request runs one actor block of `main.rs`: execute the command, answer with its `output`, publish its `events`.
- **Async event bus:** `AsyncEventBus::new(..).start()` instead of the `SyncEventBus`: every event starts as soon as it arrives, and `send(..).await` does not wait for the chain.

## CLI

```
cerne new <name>
cerne g entity <Name> [field:type ...] [field:Value1,Value2 ...] [field=Initial:Value1,Value2 ...] [id:type] [--aggregate]
cerne g value_object <Name> field:type [field:type ...]
cerne g event <Name> [field:type ...]
cerne g command <Name> [field:type ...]
cerne g read_model <Name> [field:type ...]
cerne g query <Name> [field:type ...]
cerne g port <Name>
cerne g adapter <Name> <Port>
```
