# Decisões em aberto

Cada decisão tem o mesmo formato: **a pergunta**, **por que importa**, **as opções** com prós e contras, **a recomendação** e **o que ela bloqueia** no [ROADMAP](./ROADMAP.md).

Legenda de impacto:
- 🔴 **Difícil de voltar atrás:** muda o código de todos os usuários.
- 🟡 **Média:** dá para mudar, mas custa uma versão que quebra compatibilidade.
- 🟢 **Fácil:** dá para trocar depois sem dor.

---

## Resumo

| # | Decisão | Impacto | Status | Decidido |
|---|---|---|---|---|
| D1 | Nome do projeto | 🔴 | ✅ | `cerne` (livre no crates.io) |
| D2 | Idioma do código | 🟡 | ✅ | Inglês no código, português na doc |
| D3 | Licença | 🟡 | ✅ | Apache-2.0 |
| D4 | Tipo de erro | 🔴 | ✅ | `thiserror` + `anyhow`, com categorias. Detalhes em **D16** |
| D5 | Formato do `Command` | 🔴 | ✅ | `Command<Ports>` |
| D6 | O que o `Command` retorna | 🔴 | ✅ | `Executed { output, events }` (revisto na D25) |
| D7 | O que o `then` da Policy faz | 🔴 | ✅ | Retorna o command; o `PolicyProcessor` executa. Runtime em **D17** |
| D8 | Identidade e Repository | 🟡 | ✅ | Toda `Entity` tem `id`; `Aggregate` é trait marcadora |
| D9 | Sync × Async | 🔴 | ✅ | Async com `async-trait` |
| D10 | `Send + Sync` | 🟡 | ✅ | Em tudo |
| D11 | Estrutura do projeto gerado | 🔴 | ✅ | Por camada |
| D12 | Generator edita `mod.rs`? | 🟢 | ✅ | Sim |
| D13 | Mecanismo de template | 🟢 | ✅ | Engine, com campos: `cerne g entity Order qty:i32 id:u64` |
| D14 | Read Model / Query | 🟡 | ✅ | `Query<Ports>` retornando um `ReadModel` |
| D15 | Banco e HTTP: embutir ou não? | 🔴 | ✅ | Núcleo + adapters opcionais (features) |
| D16 | Arquitetura de erros | 🔴 | ✅ | `thiserror` na estrutura, `anyhow` só em `InfrastructureError` e na borda; `DomainError` fixo |
| D17 | Runtime do `PolicyProcessor` | 🔴 | ✅ | Trait `PolicyProcessor`; padrão `tokio` + canal; Outbox Pattern |
| D18 | `DomainEvent` e `Policy` genéricos em `Ports` | 🔴 | ✅ | Consequência da D7 (Fase 1) |
| D19 | `Command` recebe `&Ports` | 🔴 | ✅ | `&Ports`, não `&mut Ports` (Fase 1) |
| D20 | O que `Policies::trigger` devolve | 🟡 | ✅ | `Vec<FiredPolicy>` (nome + command) (Fase 1) |
| D21 | Comportamento do `TokioPolicyProcessor` | 🟡 | ✅ | Canal sem limite, `on_error`, `shutdown().await` (revisto na D24) |
| D22 | Reexportar `async_trait` | 🟢 | ✅ | `cerne::async_trait` (Fase 1) |
| D23 | Nomes explícitos e id na criação | 🟡 | ✅ | `send_events`/`send_command`; command de criação não recebe id |
| D24 | Processador padrão síncrono | 🔴 | ✅ | `InlinePolicyProcessor` padrão; `TokioPolicyProcessor` com `shutdown()`; exemplos lib + bin |
| D25 | O command devolve um `Output` | 🔴 | ✅ | `type Output` + `Executed { output, events }`; policy só dispara `Output = ()` |
| D26 | O README é um tutorial da rde | 🟢 | ✅ | Walkthrough do `main.rs`; trechos verificados por `tests/readme.rs` |
| D27 | Value Objects | 🔴 | ✅ | Trait `ValueObject` só com `new` (imutável); o id de toda entidade é um value object |
| D28 | Quem decide o id | 🔴 | ✅ | O `save` devolve o id, como no Rails: `Entity::id()` é `Option` até o primeiro `save` |
| D29 | Decisões do CLI antes da Fase 2 | 🟢 | ✅ | id como value object `u64` por padrão, `--aggregate`, dependência Git |
| D30 | Como o CLI foi feito | 🟢 | ✅ | Sem `clap`; `minijinja`; `rustfmt` opcional; enum por `kind:A,B`, estado inicial por `status=A:A,B`; e2e com `clippy -D warnings` |
| D31 | Banco: o que o Cerne entrega | 🔴 | ✅ | Adapters SQLite e Postgres (sqlx); "em memória" é o SQLite em memória; `cerne new --db memory\|sqlite\|postgres`; sem MySQL |
| D32 | Outbox: o que vai para a tabela | 🔴 | ✅ | O command, serializado (serde) e registrado por nome, na mesma transação do `save` |
| D33 | HTTP: REST ou JSON-RPC | 🔴 | ✅ | Feature `axum`; `cerne new --http rest\|jsonrpc`; método `create_transfer`; ator no corpo até a fase de autenticação |
| D34 | A transação entre o agregado e a outbox | 🔴 | ✅ | `TransactionalPorts` (`begin`, `commit`, `outbox`); os sistemas externos ficam fora da transação |
| D35 | Como o CLI da Fase 3 foi feito | 🟢 | ✅ | `--policy`; repositório SQL gerado com `--aggregate`; linhas inseridas ao lado de âncoras do `cerne new` |

---

## D1 — Nome do projeto 🔴

**Pergunta:** o framework se chama `cerne`?

**Por que importa:** o nome vira o nome da crate, do binário e o `use cerne::...` no código de todo mundo. Depois de publicado, não se troca.

| Opção | A favor | Contra |
|---|---|---|
| `cerne` | Curto, significa "núcleo", combina com "core domain" | Pode estar ocupado no crates.io; pouco conhecido fora do Brasil |
| Outro nome | Pode ser mais fácil de buscar | Recomeçar a escolha |

**Recomendação:** manter `cerne`, **se** `cerne` e `cerne-cli` estiverem livres no crates.io. Verifique antes da Fase 0.

**Resposta:** Está disponível

---

## D2 — Idioma do código 🟡

**Pergunta:** nomes, mensagens e testes em inglês ou português?

**Por que importa:** hoje está misturado. `enforce_ok_quando_todas_validas` está ao lado de `trigger_fires_only_policies_whose_when_holds`. Os generators vão replicar esse padrão em todo projeto gerado.

| Opção | A favor | Contra |
|---|---|---|
| Tudo em inglês | Padrão do ecossistema Rust; alcance internacional | Barreira para quem não lê inglês |
| Tudo em português | Identidade; mais acessível no Brasil | Destoa do ecossistema; limita a adoção |
| Código em inglês, doc em português | Equilíbrio | Manter duas línguas |

**Recomendação:** código e mensagens em **inglês**; documentação e guia em **português** (com tradução depois, se o projeto crescer).

**Resposta:** Isso mesmo

---

## D3 — Licença 🟡

**Pergunta:** qual licença?

| Opção | A favor | Contra |
|---|---|---|
| MIT OR Apache-2.0 | Padrão do Rust (é a licença do próprio Rust); empresas adotam sem medo | Nenhum relevante |
| MIT | Simples | Sem proteção de patentes |
| GPL | Obriga quem usa a abrir o código | Afasta empresas |

**Recomendação:** **MIT OR Apache-2.0**.

**Resposta:** Crie um documento simples explicando a diferença e as features de cada

**Resposta final:** Apache-2.0 (depois de ler [LICENCAS.md](./LICENCAS.md)).

**Consequência a saber:** projetos sob GPL-2.0 não poderão incluir o Cerne, porque a cláusula de patentes da Apache-2.0 é incompatível com a GPL-2.0. Projetos GPL-3.0, fechados ou permissivos não são afetados.

---

## D4 — Tipo de erro 🔴

**Pergunta:** o erro continua sendo `Vec<&'static str>`?

**Por que importa:** é o tipo que aparece em **toda** assinatura do framework (`EnforcementResult`). Trocar depois quebra o código de todos os usuários.

O `&'static str` tem uma limitação: a mensagem é fixa. Não dá para gerar `"quantity 2000 exceeds 1000"`, só `"at most 1000 items"`.

| Opção | A favor | Contra |
|---|---|---|
| `Vec<&'static str>` (atual) | Simples, zero alocação, fácil de testar com `assert_eq!` | Sem mensagem dinâmica nem código de erro para API/i18n |
| `Vec<String>` | Mensagem dinâmica | Aloca; perde a ideia de "nome fixo da regra" |
| `Vec<Violation>` (struct própria) | Dá para adicionar campos depois (código, detalhes) sem quebrar ninguém | Um tipo a mais para aprender |

**Recomendação:** manter **`Vec<&'static str>`** agora (é o mais simples e explícito) e escondê-lo **sempre** atrás de `EnforcementResult`. Assim, se um dia virar `Violation`, a mudança acontece num lugar só. Para isso, `BusinessRules::check` também deve retornar `EnforcementResult<()>`.

**Resposta:** O framework usará por padrão a dobradinha thiserror e anyhow. Teremos um enum de Errors com enums para cada categoria de erro DomainError, ApplicationError, InfrastructureError ( Precisaremos discutir como ficará exatamente a arquitetura de errors, mas vai seguir esse caminho )

---

## D5 — Formato do `Command` 🔴

**Pergunta:** como um Command recebe as dependências (ports)?

**Contexto:** já combinamos que cada Command é uma struct (`PlaceOrderCommand`) e que existe um composition root `Ports`.

| Opção | Como fica | A favor | Contra |
|---|---|---|---|
| A. `Command<Ports>` (genérico) | `impl Command<Ports> for PlaceOrderCommand` | A lib não conhece a app; testes usam outro `Ports` | Um parâmetro genérico |
| B. Tipo associado | `type Ports = AppPorts;` em cada impl | — | Repetição em todo command |
| C. Ports globais | `Ports::get()` | Sem parâmetro | Escondido, difícil de testar |

**Recomendação:** **A**. Lê-se como "um command que roda sobre os ports da aplicação".

**Resposta:** Seguir recomendação.

---

## D6 — O que o `Command` retorna 🔴

**Pergunta:** o `execute` devolve um evento, vários, e de qual tipo?

**Por que importa:** no Event Storming é comum um command gerar mais de um evento (ex.: `OrderPlaced` + `StockReserved`).

| Opção | A favor | Contra |
|---|---|---|
| `Box<dyn DomainEvent>` | Simples | Só um evento por command |
| `Vec<Box<dyn DomainEvent>>` | Cobre o caso real; quem chama faz um loop em `trigger_policies` | Um `vec![]` a mais no caso comum |
| `type Event` associado | Tipado | Volta a ter um tipo por impl; não resolve múltiplos eventos |

**Recomendação:** **`EnforcementResult<Vec<Box<dyn DomainEvent>>>`**. Começar com um evento só e mudar depois quebraria todos os commands existentes.

**Resposta:** Seguir recomendação.

---

## D7 — O que o `then` de uma Policy faz 🔴

**Pergunta:** hoje o `then` é `Fn()`, ou seja, não recebe nada e não retorna nada. O que ele deveria fazer?

**Por que importa:** no Event Storming, **uma Policy dispara um Command** ("sempre que um pedido for feito, reserve o estoque"). Hoje o `then` não tem acesso aos ports nem consegue emitir um command, então o ciclo Event → Policy → Command não fecha.

| Opção | Como fica | A favor | Contra |
|---|---|---|---|
| A. `then` retorna um Command | `then: || Box::new(ReserveStockCommand { .. })` | Fiel ao Event Storming; o domínio continua sem IO; quem executa é a application | Precisa de um "executor" que rode os commands gerados |
| B. `then` recebe `&mut Ports` | `then: |ports| ...` | Direto | A policy, que mora no domínio, passa a fazer IO |
| C. Manter `Fn()` | — | Simples | O ciclo não fecha; vira só um "hook" |

**Recomendação:** **A**. `trigger_policies` passaria a devolver os commands a executar, e a application os roda. Isso mantém o domínio puro e deixa o fluxo visível.

**Resposta:** A gente pode ter um PolicyProcessor que seria um ator actix, logo, basta passar o address e dar um send no command.

---

## D8 — Identidade e Repository 🟡

**Pergunta:** onde mora o `id` de uma entidade?

**Contexto:** a trait `Entity` atual não tem `id`. Um `Repository` precisa de um `id` para carregar e salvar.

| Opção | A favor | Contra |
|---|---|---|
| Trait `Aggregate: Entity { type Id; fn id(&self) }` | Só aggregates entram em repositórios (fronteira de consistência garantida pelo compilador) | Uma trait a mais |
| `id` direto na `Entity` | Uma trait só | Toda entidade filha também precisaria de `Id` |

**Recomendação:** **trait `Aggregate`**, criada junto com o `Repository<A: Aggregate>`. 

**Resposta:** Não entendi a sua recomendação. Você tá dizendo pra aggregates terem id e entity não? Se for não faz sentido. Senão, se ambos tiverem id e ainda ganharmos a trait Aggregate ótimo.

**Esclarecimento:** a recomendação estava mal escrita. Você está certo: **toda Entity tem identidade**, é isso que a diferencia de um Value Object. Fica assim:

```rust
pub trait Entity: Sized {
    type Id: PartialEq;
    type Props;

    fn id(&self) -> &Self::Id;
    fn new(props: Self::Props) -> EnforcementResult<Self>;
    fn validate(self) -> EnforcementResult<Self>;
}

/// Marks the root of a consistency boundary: the only kind of entity a Repository can load and save.
pub trait Aggregate: Entity {}

pub trait Repository<A: Aggregate> { /* load(&A::Id), save(A) */ }
```

`Aggregate` não acrescenta métodos; ele só **marca** qual entidade é raiz. Com isso o compilador impede `Repository<OrderItem>`, porque `OrderItem` é Entity mas não é Aggregate. Os dois têm `id`, e só o aggregate ganha a trait extra. Era a sua segunda opção.

**Resposta:** Confirmado.

---

## D9 — Sync × Async 🔴 ⚠️ a mais importante

**Pergunta:** `Command::execute` e os ports são `async`?

**Por que importa:** quase todo port faz IO (banco, HTTP, fila), e em Rust o IO moderno é async (`tokio`, `sqlx`, `axum`). Se o núcleo nascer síncrono, cada command e cada port terá de ser reescrito depois.

Existe um detalhe técnico: `async fn` em traits é estável no Rust, mas traits com `async fn` **não funcionam com `dyn`**. O `Box<dyn Repository>` do composition root deixa de compilar.

| Opção | A favor | Contra |
|---|---|---|
| A. Tudo síncrono | Mais simples; zero dependências | Incompatível com `sqlx`/`axum` sem gambiarra; reescrita garantida |
| B. Async com genéricos (sem `dyn`) | Sem dependências extras; mais rápido | `Ports` vira genérico (`Ports<R: OrderRepository>`), o que é mais verboso |
| C. Async com a crate `async-trait` | Mantém o `Box<dyn ...>` no composition root; padrão bem conhecido | Uma dependência; aloca a cada chamada (irrelevante perto de IO) |

**Recomendação:** **async desde o início, com C** (`async-trait`). Domínio (Entity, Event, Invariants, Business Rules) continua **síncrono e puro**; só Command e ports são async. A linha "domínio = sync, aplicação = async" também ensina onde o IO pode acontecer.

**Resposta:** Vamos de C

---

## D10 — `Send + Sync` 🟡

**Pergunta:** os closures dos blocos exigem `Send + Sync`?

**Contexto:** hoje só `Policy` exige; `Invariant` e `BusinessRule`, não.

| Opção | A favor | Contra |
|---|---|---|
| Exigir em tudo | Funciona em servidor web multithread (`tokio`, `axum`) | Um `Rc`/`RefCell` capturado não compila |
| Em nada | Mais flexível | Não atravessa threads, o que quebra com async (D9) |

**Recomendação:** **exigir em tudo**, por consistência e porque é necessário se D9 for async.

**Resposta:** Seguir recomendação

---

## D11 — Estrutura do projeto gerado 🔴

**Pergunta:** o `cerne new` organiza o código por **camada** ou por **contexto (bounded context)**?

**Por que importa:** é a primeira coisa que o usuário vê e define onde cada generator escreve. Mudar depois quebra todo projeto existente.

**Por camada** (estilo Rails):
```
src/domain/entities/order.rs
src/domain/events/order_placed.rs
src/application/commands/place_order.rs
```

**Por contexto** (estilo DDD):
```
src/orders/domain/order.rs
src/orders/domain/order_placed.rs
src/orders/application/place_order.rs
src/billing/...
```

| Opção | A favor | Contra |
|---|---|---|
| Por camada | Familiar para quem vem do Rails; simples em projetos pequenos | Contextos se misturam; não escala; vai contra o próprio DDD |
| Por contexto | Fiel ao DDD; cada contexto pode virar uma crate depois | Generators precisam saber o contexto (`cerne g entity orders/Order`) |

**Recomendação:** **por contexto, com as camadas dentro de cada um**. Os generators recebem o contexto (`cerne g entity Order --context orders`, ou `orders/Order`), e `cerne g context orders` cria a pasta.

**Resposta:** Por camada. Com as convenções bem definidas de DDD, qualquer IA pode passar em segundos e separar os bounded contexts em crates diferentes se necessário.

---

## D12 — O generator edita o `mod.rs`? 🟢

**Pergunta:** ao criar `order.rs`, o generator também adiciona `mod order;` no `mod.rs`?

| Opção | A favor | Contra |
|---|---|---|
| Sim | O código gerado compila na hora (como o Rails faz com `routes.rb`) | Precisa editar arquivo do usuário com cuidado |
| Não, só imprime a instrução | Nunca mexe no código do usuário | Passo manual; o usuário esquece e não compila |

**Recomendação:** **sim**, adicionando apenas uma linha ao final e nunca reescrevendo o arquivo.

**Resposta:** Seguir recomendação

---

## D13 — Mecanismo de template 🟢

**Pergunta:** como os templates viram código?

| Opção | A favor | Contra |
|---|---|---|
| `str::replace` (`{{Name}}`, `{{name}}`) + crate `heck` para os casos | Zero mágica; o template é Rust quase puro | Sem `if`/loops no template |
| Engine (`tera`, `minijinja`) | Lógica no template (campos opcionais etc.) | Template fica mais difícil de ler |
| Proc-macros (`#[derive(Entity)]`) | Menos código gerado | Esconde o código, o oposto de "simples e explícito" |

**Recomendação:** **`replace` simples** agora. Trocar por `minijinja` depois é fácil se surgir a necessidade (ex.: `cerne g entity Order qty:i32 id:u64`).

**Resposta:** Engine. Quero que já nasça com a capacidade de cerne g entity Order qty:i32 id:u64`

---

## D14 — Read Model / Query 🟡

**Pergunta:** como representar o post-it verde (o que o usuário vê antes de decidir)?

| Opção | A favor | Contra |
|---|---|---|
| Trait `Query<Ports>` espelhando o `Command` (retorna dados, não eventos) | Simétrico (CQRS); fácil de ensinar | Mais um conceito |
| Deixar para o usuário | Menos código na lib | O framework fica incompleto em relação ao Event Storming |

**Recomendação:** **trait `Query<Ports>`**, com `execute` retornando `Result<T, ...>` e nunca alterando estado.

**Resposta:** Seguir recomendação. E aí cada query retornará um ReadModel.

---

## D15 — Banco e HTTP: embutir ou não? 🔴

**Pergunta:** o Cerne é só a parte de domínio, ou é um framework completo como o Rails (HTTP + banco)?

**Por que importa:** é a decisão de **escopo**. Define o tamanho do projeto, as dependências e o que "Rails para Rust" significa.

| Opção | A favor | Contra |
|---|---|---|
| Só domínio | Pequeno, sem dependências pesadas; usável com qualquer stack | Não é "Rails"; o usuário monta o resto |
| Completo e embutido | Experiência Rails de verdade | Amarra a `axum`/`sqlx`; muito mais para manter |
| Núcleo + adapters opcionais (features do Cargo: `cerne = { features = ["axum", "postgres"] }`) | O melhor dos dois; o `cerne new` escolhe o stack padrão | Mais trabalho de organização |

**Recomendação:** **núcleo + adapters opcionais**. O `cerne` continua pequeno; o `cerne new` gera um projeto já configurado com `axum` + `sqlx` por padrão, como o Rails faz com o banco.

**Resposta:** Seguir recomendação.
---

## D16 — Arquitetura de erros 🔴 (nova, a partir da D4)

**Pergunta:** como organizar `thiserror` + `anyhow` com as categorias `DomainError`, `ApplicationError` e `InfrastructureError`?

**Por que importa:** o tipo de erro aparece em toda assinatura (`Entity::new`, `Command::execute`, `Repository::load`). Ele também decide como a camada HTTP responde: um erro de domínio é `422`, um "não encontrado" é `404` e uma falha de banco é `500`.

**A regra de ouro das duas crates:**
- **`thiserror`** cria erros com **tipo**: dá para fazer `match` e decidir o que fazer.
- **`anyhow`** cria erros **sem tipo**: ótimo para "deu ruim, registra e responde 500", mas impede o `match`.

Por isso, usar `anyhow` em toda assinatura do framework apagaria a diferença entre as categorias, que é exatamente o que você quer manter.

**Proposta:**

```rust
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Domain(#[from] DomainError),
    #[error(transparent)]
    Application(#[from] ApplicationError),
    #[error(transparent)]
    Infrastructure(#[from] InfrastructureError),
}

#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    #[error("violated: {0:?}")]
    Violations(Vec<&'static str>), // o que Invariants e BusinessRules já produzem hoje
}

#[derive(Debug, thiserror::Error)]
pub enum ApplicationError {
    #[error("{0} not found")]
    NotFound(&'static str),
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct InfrastructureError(#[from] anyhow::Error); // banco, rede, fila: tudo cabe aqui
```

| Camada | Erro | HTTP | Usa |
|---|---|---|---|
| Domínio (`Entity`, `Invariants`, `BusinessRules`) | `DomainError` | 422 | `thiserror` |
| Aplicação (`Command`, `Query`) | `Error` (as três categorias) | 404 / 403 / 422 | `thiserror` |
| Infraestrutura (adapters) | `InfrastructureError` | 500 | `anyhow` por dentro |
| Borda (`main`, handlers HTTP) | — | — | `anyhow` livre |

Com isso, `EnforcementResult<T>` vira `Result<T, DomainError>`, e o `?` converte automaticamente para `Error` dentro dos commands.

**Pontos para discutir:**
1. **Categorias fixas ou da aplicação?** O `DomainError` é fixo no framework (como acima) ou cada projeto gerado define as próprias variantes (`OrderError::OutOfStock`)? Fixo é simples e os generators conseguem gerar; da aplicação é mais expressivo.
2. **Mensagens das regras:** manter `&'static str` dentro de `Violations` (simples, como hoje) ou permitir mensagem dinâmica.

**Recomendação:** a proposta acima, com **`DomainError` fixo** no início e `anyhow` apenas dentro de `InfrastructureError` e na borda.

**Resposta:** Seguir a proposta.

---

## D17 — Runtime do `PolicyProcessor` 🔴 (nova, a partir da D7)

**Pergunta:** o `PolicyProcessor` é um ator `actix`, ou outro mecanismo?

**Contexto:** a ideia da D7 é boa. A Policy decide **qual** command disparar, e um `PolicyProcessor` separado **executa**. Isso combina com a opção A (a Policy continua pura e só *retorna* o command), com o processador fazendo o `send`:

```rust
// domínio: a policy só diz o que deve acontecer
Policy::new("reserve stock", move || has_items, move || Box::new(ReserveStockCommand { order_id }))

// aplicação: o processador recebe e executa
processor.send(command).await;
```

A dúvida é só **o que** fica atrás do `processor`.

**Por que importa:** o runtime precisa conviver com o `axum` + `tokio` escolhidos na D15.

| Opção | A favor | Contra |
|---|---|---|
| A. Ator `actix` (`Addr<PolicyProcessor>`) | Modelo de ator explícito; mailbox, supervisão | Precisa do runtime `actix-rt` ao lado do `tokio` do `axum`; a crate de atores do actix evolui pouco hoje (o foco do ecossistema é o `actix-web`); `Addr` vazaria para dentro da Policy |
| B. Task `tokio` + canal (`tokio::sync::mpsc`) | Mesmo runtime do `axum`; zero dependência nova; mesma ideia de "mandar mensagem" | Supervisão e reinício ficam por nossa conta |
| C. Trait `PolicyProcessor` com A e B como adapters | Domínio e aplicação não conhecem o runtime; dá para trocar | Uma abstração a mais |

**Um alerta que vale para A e B:** a mailbox fica em memória. Se o processo cair entre o evento e o command, o command se perde. A solução clássica é o **Outbox Pattern** (gravar o command no banco na mesma transação do evento), que pode entrar na Fase 3.

**Recomendação:** **C, com B como implementação padrão**. Você mantém o modelo mental de ator ("mandar uma mensagem para o processador"), sem um segundo runtime. Se o `actix` for importante para você, ele entra como adapter alternativo sem mudar nenhuma Policy.

**Resposta:** Seguir a recomendação (C, com B como padrão), incluindo o Outbox Pattern.

---

## D18 — `DomainEvent<Ports>` e `Policy<Ports>` 🔴 (Fase 1, a partir da D7)

**Pergunta:** se o `then` da Policy retorna um command (D7), qual o tipo dele?

**Decisão:** `Box<dyn Command<Ports>>`. Com isso, `Policy`, `Policies`, `FiredPolicy` e `DomainEvent` ganham o parâmetro `Ports`, e cada evento é implementado para os ports da aplicação: `impl DomainEvent<Ports> for OrderPlaced`.

**Consequência a saber:** o arquivo do evento importa o command que a policy dispara (`domain/events/order_placed.rs` usa `application/commands/reserve_stock.rs`). O domínio conhece a **trait** `Command`, mas não faz IO: a policy só constrói o command, e quem executa é o `PolicyProcessor`.

---

## D19 — `Command::execute(&self, ports: &Ports)` 🔴 (Fase 1)

**Pergunta:** o command recebe `&Ports` ou `&mut Ports` (como no rascunho antigo)?

**Decisão:** `&Ports`. Os mesmos ports são compartilhados, via `Arc<Ports>`, entre os handlers HTTP e o `PolicyProcessor`, que rodam ao mesmo tempo. Com `&mut`, só um command poderia rodar por vez. Cada adapter cuida da própria mutabilidade (um `Mutex` em memória, um pool de conexões no banco).

---

## D20 — `Policies::trigger` devolve `Vec<FiredPolicy<Ports>>` 🟡 (Fase 1)

**Pergunta:** a coleção `Policies` continua devolvendo só nomes, como `Invariants` e `BusinessRules`?

**Decisão:** não. Ela devolve cada policy que disparou como `FiredPolicy { name, command }`, porque o command é o que importa para o processador e o nome é o que importa para teste e log. É a única exceção ao molde "a coleção devolve nomes". Quem quer só os nomes usa `PolicyProcessor::send_events`, que envia os commands e devolve os nomes.

---

## D21 — Comportamento do `TokioPolicyProcessor` 🟡 (Fase 1, a partir da D17)

**Decisões:**
- **Canal sem limite** (`mpsc::unbounded_channel`): `send` nunca espera. Um limite (backpressure) pode entrar junto com o Outbox, na Fase 3.
- **Commands encadeados rodam na mesma task**, em ordem (fila FIFO): se `ReserveStockCommand` gera `StockReserved`, e uma policy dele dispara outro command, esse command roda logo depois, sem voltar ao canal.
- **Erros vão para `on_error`**, um parâmetro de `TokioPolicyProcessor::spawn`, e a task segue para o próximo command. O Cerne não escolhe crate de log.
- ~~**A task termina** quando o último handle do processador é descartado.~~ Revisto na D24: o processador guarda o `JoinHandle`, e `shutdown().await` encerra e espera.
- **`send_events` valida antes de enviar:** se a invariante de qualquer evento falha, nenhum command é enviado.
- `tokio` (`sync`, `rt`) é dependência normal do `cerne`, não feature, porque é o processador padrão (D17).

---

## D22 — Reexportar `async_trait` 🟢 (Fase 1)

**Decisão:** o Cerne reexporta a macro como `cerne::async_trait`. Projetos gerados pelo `cerne new` escrevem `use cerne::async_trait;` e não precisam declarar a dependência `async-trait`.

---

## D23 — Nomes explícitos e id na criação 🟡 (revisão do `examples/rde`)

**Decisões:**
- **O `PolicyProcessor` tem `send_command` e `send_events`**, no lugar de `send` e `react_to`. Quem lê `processor.send_events(create_transfer_events)` sabe na hora em que parte do fluxo está.
- **Um command de criação não recebe id.** Na maioria dos casos, quem decide o id é o repositório, no insert. A exceção é um id determinístico, calculado a partir do conteúdo: no `examples/rde`, a `Transfer` é identificada pelo `tx_hash`, o Keccak-256 da transação (ver `HOW_TX_HASH_IS_GENERATED.md`), e o `Transfer::new` o calcula.
- **Variáveis com nome descritivo:** condições viram variáveis que se leem como frase (`sender_is_not_recipient`), cada evento ganha uma variável antes do `Ok(vec![...])`, e o resultado de um `execute` vai para uma variável `*_events`. As regras estão no `CLAUDE.md`.

~~**Em aberto:** o caso comum, com id gerado pelo repositório, ainda não tem forma na lib. Hoje `Entity::new` devolve a entidade com id, e `Repository::save` não devolve nada. Isso precisa ser resolvido antes dos generators (Fase 2).~~ Resolvido na D28.

---

## D24 — Processador padrão síncrono 🔴 (revisão do `examples/rde`, a partir da D17 e da D21)

**Problema:** com o `TokioPolicyProcessor` como padrão, o usuário recebia uma tupla `(processor, JoinHandle)`, sinalizava o fim com `drop(processor)` e só via os erros dos commands disparados por policy num callback `on_error`. Isso é implícito e vaza o runtime.

**Decisões:**
- **`InlinePolicyProcessor` é o padrão:** `send_events` roda na hora, com `await`, a cadeia inteira de policies e devolve o erro do primeiro command que falhar. Não há task, nem `drop`, nem `on_error`. O custo é que a requisição HTTP espera a cadeia.
- **`TokioPolicyProcessor` é a alternativa para produção:** `spawn` devolve só o processador, que guarda o `JoinHandle`, e `processor.shutdown().await` encerra e espera os commands já enviados. Os erros continuam indo para `on_error`, porque não há quem chamou para recebê-los.
- ~~**`dyn DomainEvent::downcast_ref::<E>()`** para ler o `tx_hash` do `TransferCreated`.~~ Removido na D25.
- **Os exemplos são `lib.rs` + `main.rs` + `tests/`:** o domínio, a aplicação e a infraestrutura ficam `pub` na lib, o `main` mostra um uso real do fluxo e os testes cobrem o resto. Itens públicos de uma lib não geram `dead_code`, então os `#[allow(dead_code)]` dos eventos saíram. O `cerne new` deve gerar essa mesma forma.

---

## D25 — O command devolve um `Output` 🔴 (revê a D6)

**Problema:** para devolver o `tx_hash` da transferência criada, o `main` do `examples/rde` lia `create_transfer_events[0].downcast_ref::<TransferCreated>().expect(..)`. Isso pressupõe a ordem e a quantidade dos eventos: se uma regra de negócio emitir um evento a mais, ou deixar de emitir aquele, o código lê o evento errado ou dá panic. A lista de eventos serve às policies, não ao resultado de quem chamou.

**Decisão:**

```rust
pub trait Command<Ports>: Send + Sync {
    type Output: Send;

    async fn execute(&self, ports: &Ports) -> Result<Executed<Self::Output, Ports>, Error>;
}

pub struct Executed<Output, Ports> {
    pub output: Output,
    pub events: Vec<Box<dyn DomainEvent<Ports>>>,
}
```

- Quem chama lê `execution.output` (com tipo, garantido pelo compilador) e manda `execution.events` para o processador.
- Um command sem nada a devolver usa `type Output = ();`.
- Uma policy só dispara `Box<dyn Command<Ports, Output = ()>>`, porque ninguém recebe o retorno de um command disparado por policy.
- O `downcast_ref` e o `Any` saíram de `DomainEvent`.

**Fica para a Fase 3:** o lado do destinatário. Na vida real, o Bob não recebe o `tx_hash` da Alice; ele o encontra num read model (`PendingTransfersQuery`, D14).

---

## D26 — O README é um tutorial do `examples/rde` 🟢

**Decisão:** o README percorre o `main.rs` do `examples/rde` de cima para baixo: composition root, criação pela Alice, aceite pelo Bob com a policy de encadeamento, resultado e erros. A cada passo, ele abre o código que o passo usa (command, agregado, evento, policy, port), para mostrar como o framework funciona e como uma aplicação Cerne deve ser organizada.

**Consequência:** os blocos do README deixam de ser doctests, porque o código da rde está em outro crate. Para que eles não fiquem desatualizados:
- cada bloco é uma cópia literal de um arquivo da rde, marcada com `<!-- snippet: <arquivo> -->`;
- o teste `examples/rde/tests/readme.rs` falha se algum bloco não aparecer, linha a linha, no arquivo indicado.

Os exemplos que testam a API da lib continuam como doctests, nos doc comments de cada trait.

**Complemento:** o `lib.rs` deixou de incluir o README (`include_str!`) e ganhou documentação própria, com a tabela post-it → código e links para cada trait. Com isso, os blocos do README podem usar ` ```rust ` puro, que o GitHub colore; com ` ```rust,ignore `, o GitHub mostrava o código sem cores. Isso também resolve a pendência da Fase 4: o caminho `../../../README.md` não existiria no pacote publicado.

---

## D27 — Value Objects 🔴 (Fase 1.5)

**Problema:** a Fase 1 criou Entity, Aggregate, Event, Policy e Command, mas pulou o Value Object. Sem ele, o id de toda entidade era um tipo primitivo (`u64`, `String`), e os generators da Fase 2 espalhariam isso por todo projeto gerado.

**Decisão:**

```rust
pub trait ValueObject: Sized + Clone + PartialEq + Send + Sync {
    type Props;

    fn new(props: Self::Props) -> EnforcementResult<Self>;
}
```

- **Só `new`, sem `validate`:** um value object nunca muda, então não há transição de estado para validar. Outro valor é outro value object, criado com `new`.
- **`Clone + PartialEq` na trait:** a igualdade é por valor, e o compilador garante que dá para comparar e copiar.
- As invariantes ficam no `new`, com `Invariants`, como na entidade: o erro lista todas as violadas.
- **O id de toda entidade é um value object:** `Entity::Id: ValueObject`. Na rde, o `tx_hash` virou `TxHash`, que confere o `0x` e os 64 dígitos hexadecimais.
- No projeto, os value objects ficam em `domain/value_objects/`.

---

## D28 — O `save` devolve o id 🔴 (Fase 1.5, resolve o ponto em aberto da D23)

**Pergunta:** no caso comum, em que o repositório decide o id, como o command de criação descobre o id?

| Opção | A favor | Contra |
|---|---|---|
| `next_id()` no `Repository` | A entidade nunca existe sem id | Uma chamada a mais antes de criar; não é o que o Rails faz |
| **O `save` devolve o id, e o id é opcional até lá** | Como `Order.new` + `save` no Rails; o command lê o id do `save` | Um `Option` em toda entidade e um `with_id` para o repositório |
| `insert(props)` no `Repository` | O id nunca é opcional | A entidade nasce dentro do adapter |

**Decisão:** o `save` devolve o id, como no Rails.

```rust
pub trait Entity: Sized + Send + Sync {
    type Id: ValueObject;
    type Props;

    fn id(&self) -> Option<&Self::Id>;      // None até o primeiro save
    fn with_id(self, id: Self::Id) -> Self; // só o repositório chama
    fn new(props: Self::Props) -> EnforcementResult<Self>;
    fn validate(self) -> EnforcementResult<Self>;
}

pub trait Repository<A: Aggregate>: Send + Sync {
    async fn load(&self, id: &A::Id) -> Result<A, Error>;
    async fn save(&self, aggregate: A) -> Result<A::Id, Error>; // insere se o id é None, atualiza se é Some
}
```

No command:

```rust
// --- Aggregate ---------------------------------------------------------------

let order = Order::new(OrderProps { qty: self.qty })?; // id: None

let order_id = ports.orders.save(order).await?;
```

- Uma entidade cujo id vem do próprio conteúdo (a `Transfer`, com o `tx_hash`) devolve sempre `Some` no `id()`, e o repositório só devolve esse id.
- O `InMemoryRepository` da rde só guarda agregados que já têm id. O adapter em memória da lib (Fase 3) decide o id quando ele é `None`.

---

## D29 — Decisões do CLI tomadas antes da Fase 2 🟢

Tomadas no mesmo chat da Fase 1.5, para a Fase 2 começar sem perguntas em aberto:

- **Id padrão:** `cerne g entity Order qty:i32` gera o value object `OrderId` (em `domain/value_objects/order_id.rs`) com um `u64` dentro. `id:<tipo>` nos campos troca o tipo de dentro (`id:String`).
- **`--aggregate`:** sem a flag, o generator cria só a `Entity`; com ela, também o `impl Aggregate` e as seções do molde da `Transfer`.
- **Dependência do `cerne`:** enquanto a crate não está no crates.io, o `Cargo.toml` gerado usa `cerne = { git = "https://github.com/ecdesa-labs/cerne" }`. Na Fase 4, isso vira a versão publicada.

---

## D30 — Como o CLI foi feito 🟢 (Fase 2)

Escolhas tomadas durante a Fase 2, todas na linha "a versão com menos conceitos vence":

- **Argumentos sem `clap`:** o `cerne` tem dois comandos (`new` e `g`), e um `match` sobre `env::args()` resolve. O `clap` entra quando surgirem flags com valor, ajuda por subcomando ou autocompletar.
- **Uma dependência só:** `minijinja` (D13). O `snake_case` dos nomes de arquivo são cinco linhas, então o `heck` ficou de fora.
- **Templates em `crates/cerne-cli/templates/`**, embutidos no binário com `include_str!`. O `cerne` instalado não lê nada do disco além do projeto do usuário.
- **`rustfmt` opcional:** depois de gerar um arquivo, o `cerne g` roda `rustfmt` nele. Sem `rustfmt` no PATH, o arquivo continua válido, só menos arrumado.
- **Nada é sobrescrito:** o `cerne new` falha se a pasta existe, e o `cerne g` falha se o arquivo existe ou se não acha o `mod.rs` (fora de um projeto do Cerne). O `mod.rs` só ganha `pub mod <arquivo>;` no final (D12).
- **Formato do value object:** com um campo, vira tupla com `Props` igual ao tipo do campo (`OrderId(u64)`, `OrderId::new(7)`), como o `TxHash`. Com dois ou mais, vira struct com campos privados e uma `<Nome>Props`. Sem campos, o generator recusa.
- **`--aggregate`:** a entidade ganha `impl Aggregate`, a seção `Aggregate` e a seção `State transitions` (um `impl` vazio).
- **Enum pelos valores do campo:** as vírgulas fazem o campo virar um enum, com o nome da entidade + o nome do campo, gerado numa seção própria do mesmo arquivo, como o `TransferStatus` da rde. A regra olha o tipo do campo, nunca o nome.
  - `cerne g entity Product kind:Physical,Digital`: o enum `ProductKind` entra na `ProductProps`, e quem cria o produto escolhe o valor.
  - `cerne g entity Order status=Pending:Pending,Accepted,Rejected`: o `=Pending` marca o estado inicial. O campo fica fora da `OrderProps`, e o `new` começa em `OrderStatus::Pending`, então ninguém cria um pedido já aceito. Só as transições mudam o status.
  - O valor inicial precisa estar na lista, e o `=` só vale para enum. Só o `entity` aceita esse formato: num event ou num command, o enum de outro arquivo precisaria de `use`, então o generator recusa.
- **Lugares marcados, não código inventado:** o `validate` gerado traz `Invariants::new(vec![])`, o evento traz `Policies::new(vec![])`, e o `execute` traz as seções do Event Storming vazias. O usuário preenche cada lugar.
- **Teste end-to-end:** `crates/cerne-cli/tests/new_and_generate.rs` cria um projeto num diretório temporário, roda os generators, troca a dependência Git pelo caminho de `crates/cerne` e passa `cargo clippy --all-targets -- -D warnings`. É mais rígido que o `cargo check` do roadmap: o código gerado não pode ter nenhum aviso.
- **Tipos dos campos não são importados:** `amount:Amount` gera `pub amount: Amount`, e o `use` fica com o usuário.

---

## D31 — Banco: o que o Cerne entrega 🔴 (Fase 3, revê o "sqlx + Postgres" da D15)

**Pergunta:** a D15 previa um adapter `sqlx` + Postgres. Mas quem usa o Cerne pode não querer SQL (SurrealDB, Turso, MongoDB), ou querer SQL com outro banco. O que a lib entrega, e o que fica com a aplicação?

**Por que importa:** o banco aparece em três lugares: o `Repository` de cada agregado, o read model de cada query e a tabela do Outbox (D32). Se a lib amarrar um banco, quem usa outro reescreve os três.

**Um fato que limita as opções:** o `sqlx` não é um banco, é uma crate com um driver por banco (Postgres, MySQL, SQLite). SQL muda entre eles (`$1` × `?`, tipos, `RETURNING`). Turso (`libsql`) e SurrealDB têm crates próprias, fora do `sqlx`. Então "ter só o `sqlx`" não serve para Turso nem SurrealDB: para esses, o que existe em comum é a trait `Repository`.

**Proposta em três camadas:**

| Camada | O que é | Dependência | Para quem |
|---|---|---|---|
| 0. Núcleo | Os ports (`Repository<A>`, `Outbox`) e adapters em memória (`InMemoryRepository`, `InMemoryOutbox`) | nenhuma | testes, protótipos, e a base de todo o resto |
| 1. Adapter próprio | `cerne g adapter OrderRepository --port Repository<Order>` gera o esqueleto do `impl`, e o usuário escreve as queries do banco dele | a crate do banco, no projeto do usuário | SurrealDB, Turso, MongoDB, `sqlx` com qualquer driver |
| 2. Adapters prontos | features `sqlite` e `postgres` no `cerne`: a lib traz o Outbox desses bancos (`SqliteOutbox`, `PostgresOutbox`), e o generator gera o repositório SQL de cada agregado, explícito, com a migração | `sqlx` com o driver escolhido | quem quer o caminho do Rails: `cerne new loja --db postgres` |

Na camada 2, o repositório é **código gerado no projeto**, não um tipo genérico da lib: o SQL de cada agregado fica visível e editável, como o resto do Cerne.

| Opção para a camada 2 | A favor | Contra |
|---|---|---|
| A. Repositório SQL gerado por agregado (proposta) | Explícito; colunas de verdade; o usuário edita a query | Mais código no projeto; mudar a entidade pede mudar o SQL |
| B. `JsonRepository<A>` genérico na lib: uma tabela `aggregates(kind, id, data)` com o agregado em JSON | Zero SQL para o usuário | Não dá para consultar por coluna; os read models sofrem; esconde o banco |
| C. Sem camada 2 | Lib menor | Foge do "Rails para Rust" da D15 |

**Recomendação:** as três camadas, com **A** na camada 2. A rde usa SQLite (roda em qualquer máquina e no CI, sem servidor). O adapter Postgres tem testes que rodam só com `DATABASE_URL` definida.

**Pontos para discutir:**
1. `cerne new` pergunta o banco (`--db memory|sqlite|postgres`, com `memory` como padrão) ou sempre gera em memória e o `--db` entra depois?
2. MySQL entra na camada 2 agora ou só quando alguém pedir?

**Resposta:**
1. O `cerne new` pergunta o banco: `--db memory|sqlite|postgres`, com `memory` como padrão.
2. MySQL fica de fora.
3. A transação entre o `save` e a outbox entra já (D32). Se a camada 1 (SurrealDB, Turso) atrapalhar a transação, o Cerne fica só com SQLite e Postgres.
4. **Nunca um repositório em memória feito de `Vec` ou `HashMap`.** "Em memória" é sempre o SQLite em memória, com o mesmo adapter SQL de produção. O `InMemoryRepository` que chegou a entrar na lib saiu.

**Como ficou:**
- A camada 0 não tem adapter em memória próprio: o núcleo traz os ports (`Repository`, `Outbox`), e o "em memória" é o `SqliteDatabase::in_memory()`, uma conexão única que nunca fecha.
- A camada 1 ficou com o `cerne g port` e o `cerne g adapter`, que geram a trait e o esqueleto do `impl`. A transação (D34) só existe com SQLite e Postgres.
- A camada 2 são os módulos `cerne::sqlite` (feature padrão) e `cerne::postgres` (feature `postgres`), escritos uma vez só por um `macro_rules!`: `SqliteDatabase`/`PostgresDatabase` (o pool ou uma transação aberta, com `execute`, `fetch_one`, `fetch_optional`, `fetch_all` e `migrate`), `SqliteOutbox`/`PostgresOutbox` e a função `column`. O SQL é o mesmo nos dois bancos: `$1` e `RETURNING` funcionam no SQLite. Só as migrações mudam.
- O repositório de cada agregado é código gerado no projeto (opção A), com a migração. A rde escreve o dela à mão (`SqliteTransferRepository`).
- O teste do adapter Postgres (`crates/cerne/tests/postgres.rs`) roda no CI, com um serviço Postgres, e não faz nada sem `DATABASE_URL`.

---

## D32 — Outbox: o que vai para a tabela 🔴 (Fase 3, a partir da D17)

**Pergunta:** para um command disparado por policy sobreviver a uma queda do processo, ele precisa ser gravado numa tabela. Gravar o quê?

| Opção | A favor | Contra |
|---|---|---|
| **A. O command, serializado** | O que a tabela guarda é exatamente o que vai rodar | Todo command disparado por policy ganha `Serialize` + `Deserialize` e um nome registrado |
| B. O evento, e as policies rodam de novo ao ler | Os commands não mudam | Os eventos ganham serde; o `trigger_policies` roda duas vezes |
| C. Adiar | Fase 3 menor | O risco da D17 continua |

**Resposta:** A.

**Como ficou:**
- **A API da policy não mudou:** `Policy::new(nome, when, then)` continua igual, e o `then` continua devolvendo `Box::new(UmCommand { .. })`. Uma trait nova (`PolicyCommand`, com um `NAME`) chegou a ser proposta e foi recusada: "por que mexer no que já funciona?". O que mudou é só o limite do tipo: o command que a policy dispara precisa ser `Serialize`. A `FiredPolicy` já leva o command serializado (`outbox_entry()`).
- **O nome vem do tipo:** `ChainAcceptedTransferCommand` → `chain_accepted_transfer` (`command_name`), o mesmo nome do método JSON-RPC (D33).
- A aplicação registra cada command de policy no `CommandRegistry` (`.register::<ChainAcceptedTransferCommand>()`), que lê a linha da tabela de volta (`Deserialize`). Um command fora do registro é marcado como falho, com o erro.
- A trait `Outbox` tem `store`, `next_pending`, `mark_done`, `mark_failed` e o `send_events` (o mesmo nome do `PolicyProcessor`), que dispara as policies dos eventos e guarda os commands delas.
- O `OutboxPolicyProcessor` roda cada command numa transação própria: begin, execute, guarda os commands que os eventos dele disparam, marca a linha como feita, commit. Se falhar, a transação volta atrás e a linha fica `failed`, com o erro, sem rodar de novo sozinha. `run_pending()` roda até a outbox esvaziar (testes, o `main` da rde); `run_every(intervalo, on_error)` roda para sempre numa task (servidor).
- **Garantia:** o command roda pelo menos uma vez; por isso, um command de policy precisa ser idempotente, como o `ChainAcceptedTransferCommand`, que confere `not_chained_yet`. O intervalo entre o `save` e a outbox está coberto pela transação da D34.
- O `InlinePolicyProcessor` e o `TokioPolicyProcessor` continuam na lib, sem mudança, mas os projetos gerados e a rde usam a outbox.

---

## D33 — HTTP: REST ou JSON-RPC 🔴 (Fase 3, a partir da D15)

**Pergunta:** como um command chega por HTTP?

**Resposta inicial:** uma feature HTTP no `cerne`, e o usuário escolhe entre REST e JSON-RPC. No JSON-RPC, o `cerne g command` já registra o command para ser aceito.

**Proposta:**
- **Feature `axum`** no `cerne`: `Error` vira resposta HTTP (`DomainError` → 422 com as violações, `ApplicationError::NotFound` → 404, `InfrastructureError` → 500 sem detalhes). No JSON-RPC, as mesmas categorias viram códigos de erro JSON-RPC.
- **O command é o corpo da requisição:** o command passa a derivar `Deserialize` (que a D32 já pede para os commands de policy), e o `Output` deriva `Serialize`. Um value object desserializa com `#[serde(try_from = "String")]` chamando o `new`, então as invariantes valem também na borda HTTP.
- **REST:** `cerne g endpoint CreateTransfer POST /transfers` gera o handler em `infrastructure/http/` e uma linha em `routes.rs`.
- **JSON-RPC:** `cerne new loja --http jsonrpc` gera `infrastructure/http/rpc.rs` com um `match` por método. A partir daí, cada `cerne g command CreateTransfer` acrescenta o braço `"create_transfer" => ...` ao `match`, como o D12 faz com o `mod.rs`.
- **Queries** seguem o mesmo caminho: `GET` no REST, método no JSON-RPC.

**Pontos para discutir:**
1. A escolha é no `cerne new` (`--http rest|jsonrpc`), ou um projeto pode ter os dois?
2. O nome do método JSON-RPC: `create_transfer` (snake_case do command) ou `transfers.create`?
3. O ator (quem é o remetente) vem do corpo da requisição na Fase 3, e a autenticação fica para uma fase própria?

**Resposta:**
1. A escolha é no `cerne new`: `--http rest|jsonrpc`. Um projeto não tem os dois.
2. O método JSON-RPC é o snake_case do command: `create_transfer`.
3. Na Fase 3, o ator vem no corpo da requisição. A autenticação fica para uma fase própria.

---

## D34 — A transação entre o agregado e a outbox 🔴 (Fase 3, a partir da D32)

**Pergunta:** como o `save` do agregado e a gravação dos commands na outbox entram na mesma transação, se o `Repository` e a `Outbox` são ports separados?

**Decisão:** os `Ports` da aplicação implementam `TransactionalPorts`:

```rust
#[async_trait]
pub trait TransactionalPorts: Sized + Send + Sync + 'static {
    async fn begin(&self) -> Result<Self, Error>; // os mesmos ports, escrevendo numa transação nova
    async fn commit(self) -> Result<(), Error>;   // descartar sem commit volta tudo atrás
    fn outbox(&self) -> &dyn Outbox<Self>;
}
```

- O `begin` devolve **os mesmos `Ports`**, com o repositório, as leituras e a outbox construídos sobre a transação. O command não muda: ele continua chamando `ports.transfers.save(..)`, sem saber se está numa transação.
- O fluxo fica explícito, como no Rails: `let transaction = ports.begin().await?;`, `execute(&transaction)`, `transaction.outbox.send_events(..)`, `transaction.commit().await?`. O `execute_in_transaction(command)` faz os quatro passos e é o que os handlers HTTP usam.
- **"E os outros ports?"** Os sistemas externos (blockchain, notificador, KYC) não entram na transação: não há como desfazer uma notificação enviada. O `begin` passa a eles os mesmos adapters (`Arc`). Por isso, a chamada a um sistema externo deve morar num command disparado por policy: ele roda pela outbox, pelo menos uma vez, depois que a transação do ator fez o commit.
- Uma query não abre transação: ela só lê.
- Com o SQLite em memória, o banco vive numa conexão só, e uma transação segura o banco inteiro até o commit. Dentro de uma transação, tudo precisa passar pelos ports dela, e não pelos de fora.

---

## D35 — Como o CLI da Fase 3 foi feito 🟢

- **`cerne new --db memory|sqlite|postgres --http rest|jsonrpc`**, com `memory` e sem HTTP como padrão. A escolha fica em `[package.metadata.cerne]` no `Cargo.toml`, que o `cerne g` lê. O projeto nasce com a migração da outbox (`migrations/1_create_cerne_outbox.sql`), os `Ports` com `TransactionalPorts` e o `command_registry()`, e o `main` roda as migrações (`sqlx::migrate!()`).
- **`cerne g command` × `cerne g command --policy`:** o registro da outbox só aceita commands com `Output = ()`, e um command de criação costuma devolver o id. Por isso o registro não é automático para todo command. O command de **ator** (sem flag) ganha o método no JSON-RPC; o command de **policy** (`--policy`) entra no `command_registry()` e não ganha método, porque não tem ator. É a mesma diferença do board.
- **`cerne g entity --aggregate`** gera também o repositório SQL (`infrastructure/sqlite_order_repository.rs`), a migração da tabela (`orders`) e o campo `orders` nos `Ports`. Inteiros viram `BIGINT` (com `as i64`), `f32`/`f64` viram `DOUBLE PRECISION`, `bool` vira `BOOLEAN`, `String` e os enums viram `TEXT`, e qualquer outro tipo vira `TEXT` com JSON. O banco decide ids inteiros (`RETURNING id`); com `id:String`, o `save` marca onde decidir o id. Outros tipos de id são recusados com `--aggregate`.
- **Value objects gerados derivam `Serialize`/`Deserialize`**, e o `Deserialize` passa pelo `new` (`#[serde(try_from = ..)]`): as invariantes valem na borda HTTP e na outbox. Commands derivam os dois; queries derivam `Deserialize`; read models derivam `Serialize`.
- **Revê a D12 em um ponto:** além de acrescentar `pub mod` no fim do `mod.rs`, o `cerne g` acrescenta linhas ao lado de **âncoras**, linhas que o `cerne new` escreveu: o campo `outbox` e o `outbox: Box::new(` dos `Ports`, o `CommandRegistry::new()`, o último braço `method => methods.not_found(method),` do JSON-RPC e o `.with_state(ports)` do router REST. Nada é reescrito; sem a âncora, o generator diz o que acrescentar à mão.
- **`cerne g endpoint <Nome> <MÉTODO> </caminho>`** (REST): `POST`/`PUT`/`PATCH`/`DELETE` apontam para o command `<Nome>Command`, que é o corpo; `GET` aponta para a query `<Nome>Query`, que é a query string. O handler do command é `ports.execute_in_transaction(command).await.map(Json)`.
- **`cerne g port <Nome>`** gera a trait vazia em `application/ports/`; **`cerne g adapter <Nome> <Port>`** gera o `impl` em `infrastructure/`. A ligação nos `Ports` fica com o usuário, porque um sistema externo não segue um molde.
- **Teste end-to-end:** três projetos. Memória + JSON-RPC passa no `clippy -D warnings` e roda um teste que salva, carrega e atualiza um agregado pelo repositório gerado. Postgres + REST passa no `clippy`. SQLite em arquivo, sem HTTP, passa no `clippy` e no `cargo run`, que roda as migrações geradas.
