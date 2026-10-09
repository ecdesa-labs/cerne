# Tutorial: uma loja, do board ao código

## Post-it → código

| Post-it | Conceito | No Cerne |
|---|---|---|
| 🟦 | Command | trait `Command<CompositionRoot>`, que devolve `Executed { output, events }` |
| 🟨 | Aggregate / Entity | atributos `#[entity]` e `#[aggregate]`, que escrevem as traits `Entity` e `Aggregate`; as invariantes ficam na trait `Validate` |
| — | Value Object | trait `ValueObject`, que também é o tipo do id de toda entidade; atributo `#[value_object]`, para um value object de um valor só |
| 🟧 | Domain Event | trait `DomainEvent<CompositionRoot>`; o command o grava no `EventOutbox` |
| 🟪 | Policy | `Policy` + `Policies`; o `SyncEventBus` ou o `AsyncEventBus` as executa |
| 🩷 | External System | um port (uma trait assíncrona da aplicação) e seus adapters, reunidos no `CompositionRoot` |
| 🟩 | Read Model / Query | trait `Query<CompositionRoot>`, que devolve um read model: uma struct de campos simples |
| — | Invariantes | `Invariant` + `Invariants`: o que é sempre verdade sobre uma entidade ou um value object |
| — | Regras de negócio | `BusinessRule` + `BusinessRules`: o que precisa valer para um command rodar |

## Instalação

```bash
cargo install cerne-cli
```

O Cerne precisa do Rust 1.88 ou mais novo.

## O board

O board tem dois fluxos. O cliente faz um pedido; a loja confere o estoque, e uma policy cobra o cliente. Depois, o cliente consulta o pedido.

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

Cada bloco de código abaixo é um arquivo inteiro do projeto. Um teste deste repositório roda estes comandos, escreve estes arquivos e roda `cargo test` no resultado, então o tutorial sempre compila. O código fica em inglês, como o Cerne gera.

## 1. O projeto e seus post-its

Cria o projeto `shop`.

```bash
cerne new shop
```

Entra no projeto: os comandos `cerne g` rodam dentro dele.

```bash
cd shop
```

Cria o agregado 🟨 `Order`: um pedido, com o port do seu repositório e um status que começa em `Placed`. O campo `id` o CLI cria sozinho: um value object `OrderId`, com um `u64` dentro. Com `id:<tipo>` (por exemplo, `id:String`), o `OrderId` guarda um `String` no lugar do `u64`.

```bash
cerne g entity Order product:String quantity:u32 total:u64 status=Placed:Placed,Paid --aggregate
```

Cria o evento 🟧 `OrderPlaced`: o pedido foi feito.

```bash
cerne g event OrderPlaced order_id:OrderId total:u64
```

Cria o evento 🟧 `OrderPaid`: o pedido foi pago.

```bash
cerne g event OrderPaid order_id:OrderId
```

Cria o command 🟦 `PlaceOrder`: o cliente faz um pedido.

```bash
cerne g command PlaceOrder product:String quantity:u32
```

Cria o command 🟦 `ChargeOrder`: cobrar o pedido. Quem o envia é uma policy, não um ator; para o CLI, ele é um command como outro qualquer.

```bash
cerne g command ChargeOrder order_id:OrderId total:u64
```

Cria o port 🩷 `Catalog`: o sistema externo com os preços e o estoque.

```bash
cerne g port Catalog
```

Cria o `InMemoryCatalog`: um catálogo em memória, para os testes e para este tutorial.

```bash
cerne g adapter InMemoryCatalog Catalog
```

Cria o port 🩷 `Payments`: o sistema externo que cobra o cliente.

```bash
cerne g port Payments
```

Cria o `InMemoryPayments`: um sistema de pagamentos em memória, para os testes e para este tutorial.

```bash
cerne g adapter InMemoryPayments Payments
```

Cria o read model 🟩 `OrderSummary`: o resumo do pedido que o cliente vê.

```bash
cerne g read_model OrderSummary product:String quantity:u32 total:u64 status:String
```

Cria a query `OrderSummary`: ela busca esse resumo pelo id do pedido.

```bash
cerne g query OrderSummary order_id:OrderId
```

Os campos são `nome:tipo`, e `status=Placed:Placed,Paid` cria um enum com os valores `Placed` e `Paid`, que começa em `Placed`. Depois de cada comando, o projeto continua compilando. O `--aggregate` também acrescenta o campo `order_repository` no `CompositionRoot` e, no `main.rs`, uma função `order_repository_adapter()` com um `todo!()`: o Cerne não traz adapter, e quem escreve o repositório é você.

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

Cada camada tem a sua pasta. O `domain/` é puro e síncrono: nada de IO. O `application/` é assíncrono: commands, queries e os ports que eles usam. O `infrastructure/` guarda os adapters, que você escreve: aqui, todos em memória.

Falta preencher os post-its.

## 2. Value object: `OrderId`

Um value object não tem identidade: dois `OrderId(7)` são a mesma coisa. Ele só existe se as invariantes valem, e nunca muda. O id de toda entidade é um value object, então um `0` nunca vira id de pedido, nem quando volta do JSON.

Como o `cerne g entity Order product:String quantity:u32 total:u64 status=Placed:Placed,Paid --aggregate` gerou o `src/domain/value_objects/order_id.rs`:

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

Depois de preenchido:

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

Cada invariante é um nome mais uma condição, escrita com o `invariant!`. A condição vai antes para uma variável, com o nome da frase do board. O `Invariants::enforce` roda todas e, se alguma falha, devolve `DomainError::Violations` com o nome de cada uma que falhou.

O `#[value_object]` escreve o que um value object de um valor só tem em comum: o `TryFrom<u64> for OrderId`, que passa pelo `new`, o `From<OrderId> for u64`, que devolve o `u64`, e, como o `OrderId` deriva `Serialize` e `Deserialize`, o `#[serde(try_from = "u64", into = "u64")]`. O `new`, com as invariantes, é seu. O repositório do passo 4 usa o `TryFrom` para criar o id de um pedido novo.

## 3. Agregado: `Order` 🟨

Como o `cerne g entity Order product:String quantity:u32 total:u64 status=Placed:Placed,Paid --aggregate` gerou o `src/domain/entities/order.rs`:

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

Depois de preenchido:

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

- O `#[aggregate]` escreve o que todo agregado tem em comum: o `impl Entity` (o `id()`, o `with_id` que o repositório chama e o `new`), o `impl Aggregate` e o `OrderConstructor`. Uma entidade que não é agregado (`cerne g entity` sem `--aggregate`) ganha o `#[entity]`, que escreve o mesmo menos o `impl Aggregate`: nenhum repositório a aceita.
- O `Order::new` recebe o `OrderConstructor`, com todos os campos menos dois: o id, que o repositório decide no primeiro `save` (até lá, `id()` é `None`), e o `status`, marcado com `#[skip_constructor]`. Um campo pulado começa no seu `Default`: o `OrderStatus` deriva `Default`, com `#[default]` no `Placed`, o valor inicial de `status=Placed:Placed,Paid`.
- O `validate`, no `impl Validate`, guarda as invariantes: o CLI o gera vazio, para você preencher. Ele roda no `new` e em toda transição de estado (`pay`): um pedido que quebra uma invariante nunca existe na memória.
- As transições de estado são métodos que consomem o pedido e devolvem o próximo.

## 4. Sistemas externos: ports e adapters 🩷

Um port é uma trait assíncrona da aplicação, e todo método devolve `Result<_, cerne::Error>`. O command não sabe qual adapter está por trás.

Como o `cerne g port Catalog` gerou o `src/application/ports/catalog.rs`:

<!-- generated: src/application/ports/catalog.rs -->
```rust
use cerne::async_trait;

/// External system "Catalog": one async method per thing the commands ask of it, each returning
/// `Result<_, cerne::Error>`.
#[async_trait]
pub trait Catalog: Send + Sync {}
```

Depois de preenchido:

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

Como o `cerne g port Payments` gerou o `src/application/ports/payments.rs`:

<!-- generated: src/application/ports/payments.rs -->
```rust
use cerne::async_trait;

/// External system "Payments": one async method per thing the commands ask of it, each returning
/// `Result<_, cerne::Error>`.
#[async_trait]
pub trait Payments: Send + Sync {}
```

Depois de preenchido:

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

Os adapters em memória fazem o papel dos sistemas de verdade nos testes e neste tutorial:

Como o `cerne g adapter InMemoryCatalog Catalog` gerou o `src/infrastructure/in_memory_catalog.rs`:

<!-- generated: src/infrastructure/in_memory_catalog.rs -->
```rust
use crate::application::ports::catalog::Catalog;
use cerne::async_trait;

/// An adapter of the port `Catalog`.
pub struct InMemoryCatalog;

#[async_trait]
impl Catalog for InMemoryCatalog {}
```

Depois de preenchido:

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

Como o `cerne g adapter InMemoryPayments Payments` gerou o `src/infrastructure/in_memory_payments.rs`:

<!-- generated: src/infrastructure/in_memory_payments.rs -->
```rust
use crate::application::ports::payments::Payments;
use cerne::async_trait;

/// An adapter of the port `Payments`.
pub struct InMemoryPayments;

#[async_trait]
impl Payments for InMemoryPayments {}
```

Depois de preenchido:

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

Dois ports são traits do próprio Cerne: o `Repository<Order, Transaction>`, que carrega e salva o pedido, e o `EventOutbox<Transaction>`, que grava todo evento que um command produz. Os dois recebem a transação que o command abriu. O `cerne new` escreve o `Database` e a `Transaction` dele com um `todo!()`, porque só você sabe como o seu banco abre uma transação: numa aplicação de verdade, o `Database` embrulha um pool do `sqlx`, do `diesel` ou de outro. Aqui ele guarda os pedidos e os eventos em memória, e cada `Transaction` os compartilha.

Como o `cerne new shop` gerou o `src/infrastructure/database.rs`:

<!-- generated: src/infrastructure/database.rs -->
```rust
use cerne::Error;

/// The database of the application: write it on yours (a pool of `sqlx`, `diesel` or any other). The commands only
/// call `begin`, and pass the transaction to every repository and to the event outbox.
pub struct Database;

/// One transaction of the `Database`: the type every repository and the event outbox take.
pub struct Transaction;

impl Database {
    pub async fn begin(&self) -> Result<Transaction, Error> {
        todo!("open a transaction on your database")
    }
}

impl Transaction {
    /// Makes every write of the transaction permanent; dropping it without `commit` rolls them back.
    pub async fn commit(self) -> Result<(), Error> {
        todo!("commit the transaction on your database")
    }
}
```

Depois de preenchido, com os adapters dos dois ports, que o `cerne g adapter` não escreve:

<!-- file: src/infrastructure/database.rs -->
```rust
use crate::domain::entities::order::Order;
use crate::domain::value_objects::order_id::OrderId;
use cerne::application::{EventOutbox, OutboxEntry, Repository};
use cerne::domain::Entity;
use cerne::{ApplicationError, Error, async_trait};
use std::sync::{Arc, Mutex};

/// What a database would hold, in memory.
#[derive(Default)]
pub struct Database {
    pub orders: Arc<Mutex<Vec<Order>>>,
    pub event_outbox: Arc<Mutex<Vec<OutboxEntry>>>,
}

/// In memory there is no real transaction: every write goes straight to the `Database`, and a command that fails
/// halfway keeps what it already wrote.
pub struct Transaction {
    orders: Arc<Mutex<Vec<Order>>>,
    event_outbox: Arc<Mutex<Vec<OutboxEntry>>>,
}

impl Database {
    pub async fn begin(&self) -> Result<Transaction, Error> {
        let transaction = Transaction {
            orders: Arc::clone(&self.orders),
            event_outbox: Arc::clone(&self.event_outbox),
        };

        Ok(transaction)
    }
}

impl Transaction {
    /// Nothing to make permanent: every write is already in the `Database`.
    pub async fn commit(self) -> Result<(), Error> {
        Ok(())
    }
}

// --- Repository<Order> -------------------------------------------------------

pub struct InMemoryOrderRepository;

#[async_trait]
impl Repository<Order, Transaction> for InMemoryOrderRepository {
    async fn load(&self, transaction: &mut Transaction, order_id: &OrderId) -> Result<Order, Error> {
        let orders = transaction.orders.lock().unwrap();
        let order = orders.iter().find(|order| order.id() == Some(order_id));

        Ok(order.cloned().ok_or(ApplicationError::NotFound("order"))?)
    }

    async fn save(&self, transaction: &mut Transaction, order: Order) -> Result<OrderId, Error> {
        let mut orders = transaction.orders.lock().unwrap();

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

pub struct InMemoryEventOutbox;

#[async_trait]
impl EventOutbox<Transaction> for InMemoryEventOutbox {
    async fn store(&self, transaction: &mut Transaction, outbox_entry: OutboxEntry) -> Result<(), Error> {
        transaction.event_outbox.lock().unwrap().push(outbox_entry);

        Ok(())
    }
}
```

Como o `cerne new shop`, o `cerne g adapter InMemoryCatalog Catalog` e o `cerne g adapter InMemoryPayments Payments` geraram o `src/infrastructure/mod.rs`:

<!-- generated: src/infrastructure/mod.rs -->
```rust
pub mod database;
pub mod in_memory_catalog;
pub mod in_memory_payments;
```

Depois de preenchido:

<!-- file: src/infrastructure/mod.rs -->
```rust
pub mod database;
pub mod in_memory_catalog;
pub mod in_memory_payments;
```

O `CompositionRoot` reúne todos os ports, e o `CompositionRoot::new` só guarda os adapters que recebe: quem os monta é o `main.rs`. O `cerne new` escreveu o `database` e o `event_outbox`, e o `cerne g entity --aggregate` acrescentou o `order_repository`; os dois sistemas externos entram à mão.

Nada nele abre transação: quem abre é o command, com `composition_root.database.begin()`, e ele a passa para o repositório e para o event outbox. Os sistemas externos não recebem transação, porque uma chamada a eles não tem como ser desfeita.

Como o `cerne new shop` e o `cerne g entity Order product:String quantity:u32 total:u64 status=Placed:Placed,Paid --aggregate` geraram o `src/composition_root.rs`:

<!-- generated: src/composition_root.rs -->
```rust
use crate::domain::entities::order::Order;
use crate::infrastructure::database::{Database, Transaction};
use cerne::application::{EventOutbox, Repository};

/// The composition root: the database and every port the commands and queries can use.
pub struct CompositionRoot {
    pub database: Database,
    pub order_repository: Box<dyn Repository<Order, Transaction>>,
    pub event_outbox: Box<dyn EventOutbox<Transaction>>,
}

/// What `CompositionRoot::new` takes: every adapter, already built.
pub struct CompositionRootConstructor {
    pub database: Database,
    pub order_repository: Box<dyn Repository<Order, Transaction>>,
    pub event_outbox: Box<dyn EventOutbox<Transaction>>,
}

impl CompositionRoot {
    pub fn new(constructor: CompositionRootConstructor) -> Self {
        Self {
            database: constructor.database,
            order_repository: constructor.order_repository,
            event_outbox: constructor.event_outbox,
        }
    }
}
```

Depois de preenchido:

<!-- file: src/composition_root.rs -->
```rust
use crate::application::ports::catalog::Catalog;
use crate::application::ports::payments::Payments;
use crate::domain::entities::order::Order;
use crate::infrastructure::database::{Database, Transaction};
use cerne::application::{EventOutbox, Repository};
use std::sync::Arc;

/// The composition root: the database and every port the commands and queries can use.
pub struct CompositionRoot {
    pub database: Database,
    pub order_repository: Box<dyn Repository<Order, Transaction>>,
    pub event_outbox: Box<dyn EventOutbox<Transaction>>,
    pub catalog: Arc<dyn Catalog>,
    pub payments: Arc<dyn Payments>,
}

/// What `CompositionRoot::new` takes: every adapter, already built.
pub struct CompositionRootConstructor {
    pub database: Database,
    pub order_repository: Box<dyn Repository<Order, Transaction>>,
    pub event_outbox: Box<dyn EventOutbox<Transaction>>,
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
}
```

## 5. Command: `PlaceOrderCommand` 🟦

O command é uma struct com o que o ator envia, e nada mais: sem id, porque quem decide o id é o repositório. O `execute` segue o board da esquerda para a direita, uma seção por post-it:

Como o `cerne g command PlaceOrder product:String quantity:u32` gerou o `src/application/commands/place_order.rs`:

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

        let transaction = composition_root.database.begin().await?;

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

Depois de preenchido:

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

        let mut transaction = composition_root.database.begin().await?;

        // --- Ports -----------------------------------------------------------

        let unit_price = composition_root.catalog.unit_price(&self.product).await?;
        let units_in_stock = composition_root.catalog.units_in_stock(&self.product).await?;

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

        let order_id = composition_root.order_repository.save(&mut transaction, order).await?;

        // --- Domain events ---------------------------------------------------

        let order_placed = OrderPlaced {
            order_id: order_id.clone(),
            total,
        };

        composition_root.event_outbox.store(&mut transaction, OutboxEntry::new(&order_placed)?).await?;
        transaction.commit().await?;

        Ok(Executed {
            output: order_id,
            events: vec![Box::new(order_placed)],
        })
    }
}
```

- **Transaction:** o `composition_root.database.begin()` a abre, e o repositório e o event outbox daqui para baixo recebem `&mut transaction`. O catálogo não: ele é um sistema externo.
- **Ports:** toda leitura de que o command precisa, antes de qualquer decisão.
- **Business rules:** cada condição numa variável, depois o `BusinessRules::check([business_rule!(..)])?`. Uma regra de negócio precisa do mundo de fora (aqui, o estoque); uma invariante só precisa da própria entidade.
- **Aggregate:** a mudança, depois o `save`, que devolve o id.
- **Domain events:** cada evento numa variável, gravado no event outbox com o `OutboxEntry::new`, na mesma transação do pedido: os dois são salvos, ou nenhum. Depois o `commit` e o `Ok(Executed { output, events })`. O `output` volta para quem enviou o command (aqui, o id do pedido novo), e os `events` também, que esse chamador publica no event bus (passo 6).

## 6. Evento de domínio e policy: `OrderPlaced` 🟧 🟪

Um evento diz o que aconteceu, no passado. Ele deriva `Serialize` para o command gravá-lo no event outbox, e `Deserialize` para lê-lo de volta de lá. O `trigger_policies` dele lista as policies que reagem a ele: cada uma é um `policy!` com três argumentos: um nome, uma condição (`true` numa policy que sempre dispara) e o command que ela dispara. O command só é montado se a condição vale, e ele leva os valores que usa: aqui, `order_id` e `total`, lidos do evento antes. O `Policies::trigger` devolve as policies que dispararam.

Como o `cerne g event OrderPlaced order_id:OrderId total:u64` gerou o `src/domain/events/order_placed.rs`:

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

Depois de preenchido:

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

A policy não executa o command: quem executa é o event bus. Quem enviou o `PlaceOrderCommand` publica os eventos dele com `sync_event_bus.publish(events)`, e, para cada evento, o bus chama o `trigger_policies`, executa o command de cada policy que disparou e publica os eventos que esse command devolve, até a cadeia acabar. O bus não abre transação: cada command abre a sua. O command só devolve os eventos: nunca os publica.

| | `SyncEventBus` | `AsyncEventBus` |
|---|---|---|
| Ordem | uma cadeia por vez: o próximo `publish` espera a cadeia atual acabar | cada evento começa assim que chega |
| `publish` | volta, depois do `.await`, quando a cadeia inteira rodou | volta na hora, sem `.await`: os eventos já começaram |
| Threads | uma cadeia, então um core por vez | cada evento numa task do Tokio, em todos os cores |

Os dois rodam sobre o Tokio, por isso o `main` roda em `#[tokio::main]` e os testes em `#[tokio::test]`. O `AsyncEventBus` só usa todos os cores no runtime multi-thread, o padrão do `#[tokio::main]`; o `#[tokio::test]` roda numa thread só.

O bus torce pelo melhor. Um command que falha, ou um evento cujas invariantes falham, vai para o `on_error` com que o bus foi montado, e o bus segue: nada roda de novo, e nada marca o evento. Como cada policy sobrevive a uma falha é decisão sua. Pense numa policy que manda um e-mail por uma API de notificação: se a API estiver fora do ar, o command falha, e o e-mail se perde, a não ser que você faça algo. Você pode ler de novo a tabela `event_outbox` e publicar o que ficou para trás, ou deixar o próprio command tentar de novo; aí a API pode receber o mesmo e-mail duas vezes, a não ser que ela aceite uma chave de idempotência. O Cerne grava todo evento no outbox; ler de volta é com você.

O `OrderPaid` fica como o `cerne g event` o gerou: nenhuma policy reage a ele ainda.

## 7. O command que uma policy dispara: `ChargeOrderCommand` 🟦

Como o `cerne g command ChargeOrder order_id:OrderId total:u64` gerou o `src/application/commands/charge_order.rs`:

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

        let transaction = composition_root.database.begin().await?;

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

Depois de preenchido:

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

        let mut transaction = composition_root.database.begin().await?;

        // --- Ports -----------------------------------------------------------

        let order = composition_root.order_repository.load(&mut transaction, &self.order_id).await?;

        // --- Business rules --------------------------------------------------

        let order_is_still_placed = order.status == OrderStatus::Placed;

        BusinessRules::check([business_rule!("order is still placed", order_is_still_placed)])?;

        // --- External system: Payments ---------------------------------------

        composition_root.payments.charge(&self.order_id, self.total).await?;

        // --- Aggregate -------------------------------------------------------

        let paid_order = order.pay()?;

        composition_root.order_repository.save(&mut transaction, paid_order).await?;

        // --- Domain events ---------------------------------------------------

        let order_paid = OrderPaid {
            order_id: self.order_id.clone(),
        };

        composition_root.event_outbox.store(&mut transaction, OutboxEntry::new(&order_paid)?).await?;
        transaction.commit().await?;

        Ok(Executed {
            output: (),
            events: vec![Box::new(order_paid)],
        })
    }
}
```

Se a cobrança falha, o erro vai para o `on_error` do bus, e o pedido continua `Placed`. Cobrar de novo é decisão da aplicação, e aqui é seguro: o id do pedido é a chave de idempotência do pagamento, e a regra "order is still placed" recusa um pedido que já foi pago. A seção `External system: Payments` fica entre as regras e o agregado.

## 8. Query e read model: `OrderSummary` 🟩

O read model é o que o ator vê na tela: campos simples, sem comportamento. O `cerne g read_model` o gerou, e ele fica como está. A query lê os ports e o monta; ela não muda nada, então não abre transação:

Como o `cerne g query OrderSummary order_id:OrderId` gerou o `src/application/queries/order_summary.rs`:

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

Depois de preenchido:

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
        // --- Transaction -----------------------------------------------------

        let mut transaction = composition_root.database.begin().await?;

        // --- Ports -----------------------------------------------------------

        let order = composition_root
            .order_repository
            .load(&mut transaction, &self.order_id)
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

## 9. O board como teste

O `tests/board.rs` tem um bloco por fluxo, sobre os mesmos adapters em memória do `main.rs`.

Como o `cerne new shop` gerou o `tests/board.rs`:

<!-- generated: tests/board.rs -->
```rust
//! One block per flow of the board: an actor sends a command, and the test checks its events and the policies that fired.
```

Depois de preenchido:

<!-- file: tests/board.rs -->
```rust
//! One block per flow of the board: an actor sends a command, and the test checks its events and the policies that fired.

use cerne::Error;
use cerne::application::{Command, Query, SyncEventBus};
use cerne::domain::{DomainError, ValueObject};
use shop::application::commands::place_order::PlaceOrderCommand;
use shop::application::queries::order_summary::OrderSummaryQuery;
use shop::application::read_models::order_summary::OrderSummary;
use shop::composition_root::{CompositionRoot, CompositionRootConstructor};
use shop::domain::value_objects::order_id::OrderId;
use shop::infrastructure::database::{Database, InMemoryEventOutbox, InMemoryOrderRepository};
use shop::infrastructure::in_memory_catalog::InMemoryCatalog;
use shop::infrastructure::in_memory_payments::InMemoryPayments;
use std::sync::Arc;

fn composition_root(payments: Arc<InMemoryPayments>) -> Arc<CompositionRoot> {
    let database = Database::default();

    let order_repository = InMemoryOrderRepository;
    let event_outbox = InMemoryEventOutbox;

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

#[tokio::test]
async fn the_customer_places_an_order_and_the_policy_charges_it() -> anyhow::Result<()> {
    let payments = Arc::new(InMemoryPayments::default());
    let composition_root = composition_root(Arc::clone(&payments));

    let sync_event_bus = SyncEventBus::new(Arc::clone(&composition_root), |error| eprintln!("policy: {error}"));

    // --- Customer: places an order -------------------------------------------

    let place_order = PlaceOrderCommand {
        product: "mug".into(),
        quantity: 2,
    };

    let place_order_execution = place_order.execute(&composition_root).await?;

    let order_id = place_order_execution.output;

    sync_event_bus.publish(place_order_execution.events).await;

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

#[tokio::test]
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

O teste faz o que um ator faz: executa o command, guarda o `output` e publica os `events` no `SyncEventBus`. O `publish(..).await` volta quando a cadeia inteira rodou, então a cobrança já está lá na linha seguinte.

```bash
cargo test
```

## 10. A aplicação: `main.rs`

O `main.rs` monta todos os adapters, o composition root e o event bus, e depois tem um bloco por ator. O `cerne new` escreve uma função com um `todo!()` para cada adapter que falta; no arquivo preenchido, elas dão lugar aos adapters em memória.

Como o `cerne new shop` e o `cerne g entity Order product:String quantity:u32 total:u64 status=Placed:Placed,Paid --aggregate` geraram o `src/main.rs`:

<!-- generated: src/main.rs -->
```rust
use cerne::application::{EventOutbox, Repository, SyncEventBus};
use shop::composition_root::{CompositionRoot, CompositionRootConstructor};
use shop::domain::entities::order::Order;
use shop::infrastructure::database::{Database, Transaction};
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // --- Composition root ----------------------------------------------------

    let database = Database;

    let order_repository = order_repository_adapter();
    let event_outbox = event_outbox_adapter();

    let composition_root_constructor = CompositionRootConstructor {
        database,
        order_repository,
        event_outbox,
    };

    let composition_root = Arc::new(CompositionRoot::new(composition_root_constructor));

    // --- Event bus: the policies of every event ------------------------------

    #[expect(unused_variables, reason = "the blocks of the actors publish their events on it")]
    let sync_event_bus = SyncEventBus::new(composition_root, |error| eprintln!("policy: {error}"));

    // One block per actor: execute the command, then publish its events with
    // `sync_event_bus.publish(execution.events).await;`.

    Ok(())
}

/// No adapter of EventOutbox yet: write one in `infrastructure/` and build it here.
fn event_outbox_adapter() -> Box<dyn EventOutbox<Transaction>> {
    todo!("an adapter of EventOutbox")
}

/// No adapter of Repository<Order> yet: write one in `infrastructure/` and build it here.
fn order_repository_adapter() -> Box<dyn Repository<Order, Transaction>> {
    todo!("an adapter of Repository<Order>")
}
```

Depois de preenchido:

<!-- file: src/main.rs -->
```rust
use cerne::application::{Command, Query, SyncEventBus};
use shop::application::commands::place_order::PlaceOrderCommand;
use shop::application::queries::order_summary::OrderSummaryQuery;
use shop::composition_root::{CompositionRoot, CompositionRootConstructor};
use shop::infrastructure::database::{Database, InMemoryEventOutbox, InMemoryOrderRepository};
use shop::infrastructure::in_memory_catalog::InMemoryCatalog;
use shop::infrastructure::in_memory_payments::InMemoryPayments;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // --- Composition root ----------------------------------------------------

    let database = Database::default();

    let order_repository = InMemoryOrderRepository;
    let event_outbox = InMemoryEventOutbox;

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

    let sync_event_bus = SyncEventBus::new(Arc::clone(&composition_root), |error| eprintln!("policy: {error}"));

    // --- Customer: places an order -------------------------------------------

    let place_order = PlaceOrderCommand {
        product: "mug".into(),
        quantity: 2,
    };

    let place_order_execution = place_order.execute(&composition_root).await?;

    let order_id = place_order_execution.output;

    sync_event_bus.publish(place_order_execution.events).await;

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

O pedido já está `Paid`: o `SyncEventBus` executou o `ChargeOrderCommand` antes de o `publish(..).await` voltar. Um servidor web, um consumidor de fila ou um CLI ficariam no lugar desses blocos: cada um executa o command e publica os eventos dele, do mesmo jeito.

## 11. Quando algo dá errado

Todo erro é um `cerne::Error`, numa de três categorias. Um adapter de HTTP (ou de qualquer outra coisa) olha a categoria para decidir a resposta:

| Erro | Quando |
|---|---|
| `DomainError::Violations` | uma invariante ou uma regra de negócio falhou; ele traz todos os nomes que falharam, como estão escritos no board |
| `ApplicationError::NotFound` | o repositório (ou um adapter) não achou nada |
| `InfrastructureError` | banco, rede, fila |

No teste do passo 9, o `PlaceOrderCommand { product: "mug".into(), quantity: 11 }` volta como `DomainError::Violations(["stock covers the quantity"])`.

## Outras opções

- **Um banco de verdade:** escreva sobre ele o `Database` e a `Transaction` do `src/infrastructure/database.rs` (com o `sqlx`, o `diesel` ou outro qualquer), e os adapters do `Repository<Order, Transaction>` e do `EventOutbox<Transaction>` sobre essa transação. A tabela `event_outbox` é sua: lê-la de volta para publicar o que ficou para trás é como uma policy sobrevive a uma queda.
- **HTTP, uma fila, um CLI:** cada requisição roda um bloco de ator do `main.rs`: executa o command, responde com o `output` dele e publica os `events`.
- **Event bus assíncrono:** o `AsyncEventBus::new(..)` no lugar do `SyncEventBus`: cada evento começa numa task própria assim que chega, e o `async_event_bus.publish(..)` volta sem esperar a cadeia.

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
