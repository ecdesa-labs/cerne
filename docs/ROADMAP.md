# Roadmap do Cerne

> **Objetivo:** um framework para Rust, no espírito do Ruby on Rails, em que cada post-it do Event Storming vira código explícito. Um CLI (`cerne new`, `cerne g ...`) gera o projeto e os blocos DDD.

As decisões citadas como **D1, D2...** estão em [DECISOES.md](./DECISOES.md).

---

## Onde estamos hoje

O que já existe no código, comparado com os elementos do Event Storming:

| Post-it | Conceito | Status | Onde |
|---|---|---|---|
| 🟦 Azul | Command | ✅ Pronto (trait `Command<Ports>`, async, `Executed { output, events }`) | `crates/cerne/src/commands.rs` |
| 🟨 Amarelo | Aggregate / Entity | ✅ Pronto (traits `Entity` com `id` opcional até o primeiro `save` e `Aggregate`) | `crates/cerne/src/entities.rs` |
| — | Value Object | ✅ Pronto (trait `ValueObject`, também o tipo do id de toda entidade) | `crates/cerne/src/value_objects.rs` |
| 🟧 Laranja | Domain Event | ✅ Pronto (trait `DomainEvent<Ports>`) | `crates/cerne/src/domain_events.rs` |
| 🟪 Lilás | Policy | ✅ Pronto (`Policy` + `Policies`, o `then` retorna o command, já serializado para a outbox) | `crates/cerne/src/policies.rs` |
| — | Policy Processor | ✅ Pronto (`OutboxPolicyProcessor`, o padrão dos projetos gerados; `InlinePolicyProcessor` e `TokioPolicyProcessor` continuam) | `crates/cerne/src/outbox.rs`, `policy_processors.rs` |
| — | Outbox e transação | ✅ Pronto (`Outbox`, `TransactionalPorts`, `CommandRegistry`) | `crates/cerne/src/outbox.rs` |
| — | Banco | ✅ Pronto (`cerne::sqlite`, também em memória, e `cerne::postgres`) | `crates/cerne/src/sql.rs` |
| — | HTTP | ✅ Pronto (feature `axum`: REST e JSON-RPC) | `crates/cerne/src/http.rs` |
| 🩷 Rosa | External System (Ports) | ✅ `Repository<A>` (o `save` devolve o id) + composition root `Ports`; o repositório SQL de cada agregado é gerado no projeto | `crates/cerne/src/repositories.rs` |
| — | Invariants | ✅ Pronto | `crates/cerne/src/invariants.rs` |
| — | Business Rules | ✅ Pronto | `crates/cerne/src/business_rules.rs` |
| — | Erros | ✅ Pronto (`Error` → `DomainError` / `ApplicationError` / `InfrastructureError`) | `crates/cerne/src/errors.rs` |
| 🟩 Verde | Read Model / Query | ✅ Pronto (trait `Query<Ports>` que devolve um `ReadModel`) | `crates/cerne/src/queries.rs` |
| 🟨 Amarelo pequeno | Actor | ❌ Não existe | — |
| — | CLI (`cerne new`, `cerne g`) | ✅ Pronto (`--db`, `--http`; entity com repositório, value_object, event, command, read_model, query, endpoint, port, adapter) | `crates/cerne-cli/` |

**Próximo passo:** a Fase 3.5, que roda de verdade o que a Fase 3 só compilou (Postgres e REST); depois, a Fase 4, publicação.

---

## Fase 0 — Fundação do repositório ✅

**Meta:** o repositório tem a forma final, e qualquer pessoa clona, roda `cargo test --workspace` e vê tudo verde.

- [x] A pasta `cerne/` é a raiz: um workspace virtual com `members = ["crates/*", "examples/*"]`.
- [x] Biblioteca em `crates/cerne/`.
- [x] Projeto de exemplo em `examples/shop/` (por enquanto, só um "Hello, world!").
- [x] Versão, edição e licença compartilhadas em `[workspace.package]`.
- [x] CI no GitHub Actions: `fmt --check`, `clippy -D warnings` e `test --workspace`.
- [x] README com o mapa post-it → código, os exemplos, a seção "Licença" e a frase sobre o código gerado.
- [x] Os exemplos do README são testados (doctests), então não ficam desatualizados.
- [x] Licença Apache-2.0: arquivo `LICENSE` e `license` no `Cargo.toml` (**D3** ✅).
- [x] Testes e mensagens em inglês (**D2** ✅).

**Decisões:** D1, D2 e D3 ✅.

**Pronto quando:** `cargo test --workspace` roda todos os testes da lib e o CI passa.

---

## Fase 1 — Núcleo estável (a lib `cerne`) ✅

**Meta:** todos os post-its centrais existem como código, com a mesma "cara": simples, explícito e legível de cima para baixo.

- [x] Arquitetura de erros com `thiserror` + `anyhow`: `Error` → `DomainError` / `ApplicationError` / `InfrastructureError` (**D4**, **D16** ✅).
- [x] `EnforcementResult` usando `DomainError`, também em `BusinessRules::check`.
- [x] `id` na trait `Entity` + trait marcadora `Aggregate` + port `Repository<A: Aggregate>` (**D8** ✅).
- [x] Trait `Command<Ports>` async (`async-trait`), retornando `Executed { output, events }` (D25, revendo a D6), + composition root `Ports` (**D5**, **D6**, **D9** ✅).
- [x] O `then` da Policy retorna o command a disparar; trait `PolicyProcessor` com implementação padrão `tokio` + `mpsc` (**D7**, **D17** ✅).
- [x] Desenhar a trait `PolicyProcessor` já pensando no Outbox: o processador recebe commands, e de onde eles vêm (canal ou tabela) é detalhe da implementação.
- [x] Padronizar `Send + Sync` em todos os blocos (**D10** ✅).
- [x] Doc comments com exemplo em cada trait (aparecem no docs.rs).
- [x] Exemplo implementando um fluxo completo. Começou no `examples/shop` (`PlaceOrderCommand` → `Order` → `OrderPlaced` → policy), que foi removido; hoje é o `examples/rde`: `CreateTransferCommand` → `Transfer` → `TransferAcceptedByRecipient` → policy → `ChainAcceptedTransferCommand`.

**Decisões novas:** D18 a D26.

**Pronto quando:** o `examples/rde` executa o fluxo completo do Event Storming, e cada conceito cabe numa tela de código.

---

## Fase 1.5 — Value Objects e identidade ✅

**Meta:** o id de toda entidade é um value object, e o repositório decide o id no `save`, como no Rails. Os generators da Fase 2 dependem disso.

- [x] Trait `ValueObject` (só `new`, imutável), no molde das demais: struct própria + trait da lib, com invariantes (**D27** ✅).
- [x] `Entity::Id: ValueObject`; `Entity::id()` devolve `Option<&Id>` (`None` até o primeiro `save`) e `Entity::with_id` recebe o id decidido pelo repositório (**D28** ✅).
- [x] `Repository::save` devolve `A::Id`: insere quando o id é `None`, atualiza quando é `Some` (**D28** ✅). Resolve o ponto em aberto da D23.
- [x] `examples/rde`: o id da `Transfer` virou o value object `TxHash` (em `domain/value_objects/`), e o `CreateTransferCommand` pega o `tx_hash` do `save`.
- [x] README: seção do value object `TxHash`, com a comparação Entity × Value Object, e o `save` que devolve o id.
- [x] Decisões do CLI tomadas antes da Fase 2 registradas na **D29**.

**Decisões novas:** D27, D28 e D29.

**Pronto quando:** a lib e a rde compilam com ids como value objects, e o `save` devolve o id.

---

## Fase 2 — CLI e generators ✅

**Meta:** `cerne new loja` cria um projeto que compila, e `cerne g ...` adiciona blocos que compilam na hora.

- [x] Crate `crates/cerne-cli` com o binário `cerne`.
- [x] O `Cargo.toml` gerado depende do `cerne` pelo Git do GitHub até a publicação (Fase 4) (**D29** ✅).
- [x] `cerne new <nome>`: o boilerplate **por camada** (**D11** ✅) com `domain/`, `application/`, `infrastructure/` e `ports.rs`.
- [x] Engine de template (ex.: `minijinja`) com parser de campos `nome:tipo` (**D13** ✅).
- [x] `cerne g entity Order qty:i32 id:u64`: o id vira o value object `OrderId` em `domain/value_objects/`; sem `id:<tipo>`, o generator usa `u64`. Com `--aggregate`, gera também o `impl Aggregate` (**D29** ✅).
- [x] `cerne g value_object Amount value:u64`
- [x] `cerne g event OrderPlaced order_id:u64`
- [x] O `cerne new` gera `lib.rs` (domínio, aplicação, infraestrutura e ports, tudo `pub`) + `main.rs` + `tests/`, como os exemplos. Assim, um evento que nenhuma policy lê não gera aviso de `dead_code` (D24).
- [x] `cerne g command PlaceOrder order_id:u64 qty:i32`
- [x] Registro automático do `mod` no `mod.rs` correspondente (**D12** ✅).
- [x] Teste end-to-end: gera um projeto num diretório temporário, roda os generators e executa `cargo check`.

**Decisões novas:** D30.

**Pronto quando:** `cerne new loja && cd loja && cerne g entity Order qty:i32 && cargo check` passa sem tocar em nada.

---

## Fase 3 — Do domínio ao mundo real ✅

**Meta:** dá para construir uma aplicação de verdade, com banco e HTTP.

- [x] Trait `Query<Ports>` retornando um `ReadModel`, o post-it verde (**D14** ✅). Na rde, a `PendingTransfersQuery` é como o Bob acha a transferência da Alice (D25).
- [x] `cerne g query <Nome>` e `cerne g read_model <Nome> campo:tipo`.
- [x] Outbox Pattern, para que um command disparado por policy não se perca se o processo cair (**D17**, **D32**): o command vai para a tabela `cerne_outbox` na mesma transação do agregado (**D34**), e o `OutboxPolicyProcessor` o executa.
- [x] Repositório em memória para testes e protótipos, que decide o id no `save` (D28): é o SQLite em memória (`SqliteDatabase::in_memory()`), com o mesmo adapter SQL de produção (**D31**). Nunca um `Vec`.
- [x] Adapters de banco `sqlx`: SQLite (feature padrão) e Postgres (feature `postgres`), com a mesma API e o mesmo SQL (**D15**, **D31**).
- [x] Integração HTTP (feature `axum`, **D33**): REST com `cerne g endpoint`, ou JSON-RPC, em que `cerne g command` e `cerne g query` acrescentam o método.
- [x] `cerne g port <Nome>` e `cerne g adapter <Nome> <Port>`.
- [x] `cerne new --db memory|sqlite|postgres --http rest|jsonrpc`, e o `cerne g entity --aggregate` gera o repositório SQL, a migração e o campo nos `Ports` (**D35**).

**Decisões novas:** D31 a D35.

**Pronto quando:** o `examples/rde` expõe uma API HTTP que persiste transferências num banco. ✅ O binário `server` da rde atende JSON-RPC em `POST /rpc` e grava num SQLite em arquivo; a transferência sobrevive a um reinício.

---

## Fase 3.5 — Verificar o que a Fase 3 só compilou

**Meta:** tudo o que a Fase 3 entregou roda de verdade, não só passa no `clippy`.

- [ ] Ver o job `postgres` do CI passar. O adapter Postgres (`cerne::postgres`) compila e passa no `clippy`, mas nunca rodou contra um Postgres de verdade: a máquina da Fase 3 não tinha Postgres nem Docker. O teste `crates/cerne/tests/postgres.rs` só roda com `DATABASE_URL`; localmente, com um Postgres no ar:

  ```bash
  DATABASE_URL=postgres://postgres:cerne@localhost:5432/postgres cargo test -p cerne --no-default-features --features postgres --test postgres
  ```

  Ele cobre a outbox (`FOR UPDATE SKIP LOCKED`, rollback e commit).
- [ ] Executar o repositório que o `cerne g entity --aggregate` gera para Postgres. Hoje ele só passa pelo `clippy` no e2e.
- [ ] Executar o REST gerado: no e2e, mandar um `POST` e um `GET` ao projeto `--http rest`, como o teste de ida e volta que já existe para o repositório. Hoje o handler gerado pelo `cerne g endpoint` só passa pelo `clippy`; a rde usa JSON-RPC.

- [ ] Acrescentar HTTP a um projeto que nasceu sem `--http`: hoje não há `cerne g http rest|jsonrpc`, e o `cerne g endpoint` recusa o projeto. Quem muda de ideia cria a camada à mão.
- [ ] JSON-RPC com `params` por posição (array) e por nome (objeto) (**D36** ✅). Por posição é o que a MetaMask manda ([METAMASK.md](./METAMASK.md)).
  - `crates/cerne/src/http.rs`: o `serde` já lê um array numa struct de command, na ordem dos campos. O `eth_sendRawTransaction` da rde depende disso (`["0x02f8…"]` vira o `CreateTransferCommand`). Falta o teste na lib, para o `Methods::command` e o `Methods::query`; um `params` que não é array nem objeto volta `-32602`.
  - `examples/rde/tests/http.rs`: já cobertos o `params` em array para um command (`eth_sendRawTransaction`), o `params` ausente (`net_version`) e o `id` como string. Faltam o array para uma query, uma posição que leva um objeto (como o `eth_call`) e o `id` como número grande.
  - Documentar que a ordem dos campos do command é contrato: no doc comment do `Methods`, no `rpc.rs.jinja` e no `command.rs.jinja` do CLI.
  - README, passo 7: o `curl` com `params` em array.
- [ ] JSON-RPC: a `Request` ganha o campo `jsonrpc`, e um valor diferente de `"2.0"` (ou a ausência dele) volta `-32600` (**D36**).
- [ ] JSON-RPC: o handler lê o corpo cru com `serde_json`, em vez do extractor `Json<Request>` do axum. JSON malformado volta `-32700`, e um envelope sem `method` volta `-32600`, sempre com HTTP 200 e `"id": null` (**D36**). A lib dá a função que transforma o corpo numa `Request` ou num `ErrorObject`; o `rpc.rs` da rde e o `rpc.rs.jinja` do CLI passam a usá-la. Testes: corpo que não é JSON, JSON sem `method` e `"jsonrpc": "1.0"`.

### A rde chamada pela MetaMask (D38 ✅)

Feito no começo da Fase 3.5. A rde fala o JSON-RPC da Ethereum ([METAMASK.md](./METAMASK.md)). Uma transação assinada por uma MetaMask de verdade (a capturada em 2026-10-07) passou pelo servidor: criação, aceite, encadeamento e recibo com `status: "0x1"`.

- [x] `eth_sendRawTransaction` é o `CreateTransferCommand`, cujo único campo é a `SignedTransaction`: um value object que decodifica a transação EIP-1559 (crate `alloy`), recupera o remetente da assinatura e confere a chain (8808). O `create_transfer` saiu do JSON-RPC.
- [x] O `tx_hash` é o keccak dos bytes assinados, o mesmo que a MetaMask calcula. O encadeamento manda à chain a transação como chegou.
- [x] Valores em wei (`u128`); no banco e no read model, como texto. O `Address` é um value object (`0x` + 40 dígitos, em minúsculas).
- [x] Taxas: o gas que a transação assinada permite (gas limit × max fee per gas) mais 1% de descarbonização por cima.
- [x] Regras novas na criação: "nonce is the next one of the sender" e "sender has no open transfer" (um envio em aberto por remetente, lido pelo `OpenTransfersQuery`). Elas resolvem os hotspots 1 e 5.
- [x] Rejeitada ou cancelada, a transferência entra na chain como falha (`ChainFailedTransferCommand`): recibo com `status: "0x0"`, nenhum saldo muda, e o nonce anda. A MetaMask para de esperar e libera o próximo envio.
- [x] O port `Blockchain` ganhou blocos, recibos e o preço do gas; o `InMemoryBlockchain` põe cada transação num bloco próprio. O `eth.rs` responde os métodos de leitura da MetaMask a partir dele.
- [x] O servidor lê o `RDE_WALLETS`: as carteiras que começam com 1000 RDEC e com KYC.
- [ ] O "Cancelar" da MetaMask: ela manda outra transação com o mesmo nonce, valor 0 e taxa 10% maior. Mapear para o `CancelTransferCommand`, com a regra de que só se cancela o que ainda não foi incluído numa proposta de bloco. A MetaMask passa a acompanhar o hash da transação nova.
- [ ] A mensagem do `-32001`: a MetaMask mostra só o `message` do erro ("the domain refused the request"), e as violações ficam no `data`, onde a pessoa não as vê. Pôr os nomes das violações no `message`.
- [ ] A chain em memória recomeça a cada vez que o servidor sobe, mas o `rde.db` fica. Hoje é preciso apagar o `rde.db` ao reiniciar.

**Pronto quando:** o CI roda o Postgres, o e2e executa os handlers REST gerados e o JSON-RPC segue a especificação nos três itens da D36.

---

## Fase 4 — Publicação

- [ ] Publicar (nomes livres, D1 ✅) `cerne` e `cerne-cli` (0.1.0).
- [ ] Trocar a dependência Git do `cerne new` pela versão publicada (D29).
- [x] Guia "do Event Storming ao código": o README é um tutorial que percorre o `main.rs` do `examples/rde` (D26).
- [ ] Política de versionamento: enquanto estiver em 0.x, mudanças que quebram compatibilidade são permitidas e registradas no CHANGELOG.
- [ ] Definir a MSRV (versão mínima do Rust suportada).
- [x] ~~O `lib.rs` inclui o README da raiz com `include_str!("../../../README.md")`.~~ Resolvido na D26: o `lib.rs` tem a própria documentação. Como o pacote publicado só contém `crates/cerne/`, esse caminho quebra no `cargo publish`. Antes de publicar, copiar o README para dentro da crate (ou apontar `readme` no `Cargo.toml`).

---

## Fase 5 — O domínio em outras linguagens

**Meta:** o domínio de um projeto Cerne (value objects, entidades, invariantes, regras de negócio) vira biblioteca para TypeScript e Elixir, com as mesmas invariantes nos dois lados (**D37**).

**Decidido na D37:**
- **O que é exportado:** tudo, menos o que recebe `Ports`. Value objects e entidades vão inteiros (struct, `new`, invariantes, transições). Dos commands vão a struct e o `business_rules`; dos eventos, a struct.
- **A fronteira:** tipos nativos de cada alvo (classes com `.d.ts` no TypeScript, structs no Elixir), não JSON. Quem chama recebe tipos de verdade, e a ordem dos campos do command, que é contrato desde a D36, vem do Rust.
- **Onde mora:** uma crate separada e gerada, que embrulha o domínio. Os atributos de cada alvo (`#[wasm_bindgen]`, `NifStruct`) e o `serde` dos eventos ficam no wrapper; o domínio não ganha nenhum.
- **Os alvos, em ordem:** TypeScript via WebAssembly (`wasm-bindgen` + `wasm-pack`, navegador e Node), depois Elixir via Rustler (NIFs). O Node nativo via `napi-rs` fica de fora.

- [x] Decidir a **D37**.
- [x] Regras de negócio num método síncrono `business_rules` do command, chamado pelo `execute`: nos commands da rde, no exemplo da doc do `Command` e no `command.rs.jinja` do CLI. Feito no começo da Fase 3.5.
- [ ] Generator da crate que embrulha o domínio (`cerne g bindings --wasm`, depois `--elixir`), com um tipo nativo por value object, entidade, command e evento.
- [ ] TypeScript via WebAssembly: pacote npm com as classes e os `.d.ts`. As violações chegam como uma exceção com os nomes, como `["amount is positive"]`.
- [ ] Elixir via Rustler: as violações chegam como `{:error, ["amount is positive"]}`.
- [ ] Na rde: o `TxHash`, a `Transfer` e o `CreateTransferCommand::business_rules` dão em TypeScript e em Elixir as mesmas violações do Rust.

**Fica de fora:** tudo o que recebe `Ports`: o `execute` dos commands, o `trigger_policies` dos eventos, os repositórios e a outbox.

---

## Fase 6 — Skills para agentes

**Meta:** qualquer agente (Claude Code e outros) sabe usar todo o potencial do Cerne e escreve código legível no estilo do projeto, sem depender de quem já conhece o código.

- [ ] Skills que ensinam o fluxo do Event Storming ao código: cada post-it, o generator que o cria (`cerne g ...`) e a seção do `execute` onde ele mora.
- [ ] As convenções de estilo do `CLAUDE.md` viram parte das skills, com exemplos tirados da rde.

Escopo detalhado ainda a definir.

---

## Decisões pendentes

Nenhuma. A D36, a D37 e a D38 foram decididas no começo da Fase 3.5; a D36 e a D38, com base na captura da MetaMask. O ponto em aberto da D23 (id gerado pelo repositório) foi resolvido na D28. Ver o resumo em [DECISOES.md](./DECISOES.md).
