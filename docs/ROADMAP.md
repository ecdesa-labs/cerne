# Roadmap do Cerne

> **Objetivo:** um framework para Rust, no espírito do Ruby on Rails, em que cada post-it do Event Storming vira código explícito. Um CLI (`cerne new`, `cerne g ...`) gera o projeto e os blocos DDD.

As decisões citadas como **D1, D2...** estão em [DECISOES.md](./DECISOES.md).

---

## Onde estamos hoje

O que já existe no código, comparado com os elementos do Event Storming:

| Post-it | Conceito | Status | Onde |
|---|---|---|---|
| 🟦 Azul | Command | ✅ Pronto (trait `Command<Ports>`, async, `Executed { output, events }`) | `crates/cerne/src/commands.rs` |
| 🟨 Amarelo | Aggregate / Entity | ✅ Pronto (traits `Entity` com `id` e `Aggregate`) | `crates/cerne/src/entities.rs` |
| 🟧 Laranja | Domain Event | ✅ Pronto (trait `DomainEvent<Ports>`) | `crates/cerne/src/domain_events.rs` |
| 🟪 Lilás | Policy | ✅ Pronto (`Policy` + `Policies`, o `then` retorna o command) | `crates/cerne/src/policies.rs` |
| — | Policy Processor | ✅ Pronto (trait `PolicyProcessor`; `InlinePolicyProcessor` padrão e `TokioPolicyProcessor`) | `crates/cerne/src/policy_processors.rs` |
| 🩷 Rosa | External System (Ports) | ✅ `Repository<A>` + composition root `Ports`; outros ports na Fase 3 | `crates/cerne/src/repositories.rs` |
| — | Invariants | ✅ Pronto | `crates/cerne/src/invariants.rs` |
| — | Business Rules | ✅ Pronto | `crates/cerne/src/business_rules.rs` |
| — | Erros | ✅ Pronto (`Error` → `DomainError` / `ApplicationError` / `InfrastructureError`) | `crates/cerne/src/errors.rs` |
| 🟩 Verde | Read Model / Query | ❌ Fase 3 | — |
| 🟨 Amarelo pequeno | Actor | ❌ Não existe | — |

**Próximo passo:** a Fase 2, o CLI `cerne` e os generators.

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

## Fase 2 — CLI e generators

**Meta:** `cerne new loja` cria um projeto que compila, e `cerne g ...` adiciona blocos que compilam na hora.

- [ ] Crate `crates/cerne-cli` com o binário `cerne`.
- [ ] `cerne new <nome>`: o boilerplate **por camada** (**D11** ✅) com `domain/`, `application/`, `infrastructure/` e `ports.rs`.
- [ ] Engine de template (ex.: `minijinja`) com parser de campos `nome:tipo` (**D13** ✅).
- [ ] `cerne g entity Order qty:i32 id:u64`
- [ ] `cerne g event OrderPlaced order_id:u64`
- [ ] O `cerne new` gera `lib.rs` (domínio, aplicação, infraestrutura e ports, tudo `pub`) + `main.rs` + `tests/`, como os exemplos. Assim, um evento que nenhuma policy lê não gera aviso de `dead_code` (D24).
- [ ] `cerne g command PlaceOrder order_id:u64 qty:i32`
- [ ] Registro automático do `mod` no `mod.rs` correspondente (**D12** ✅).
- [ ] Teste end-to-end: gera um projeto num diretório temporário, roda os generators e executa `cargo check`.

**Pronto quando:** `cerne new loja && cd loja && cerne g entity Order qty:i32 && cargo check` passa sem tocar em nada.

---

## Fase 3 — Do domínio ao mundo real

**Meta:** dá para construir uma aplicação de verdade, com banco e HTTP.

- [ ] Trait `Query<Ports>` retornando um `ReadModel`, o post-it verde (**D14** ✅).
- [ ] `cerne g query <Nome>` e `cerne g read_model <Nome> campo:tipo`.
- [ ] Outbox Pattern, para que um command disparado por policy não se perca se o processo cair (**D17**).
- [ ] Adapter em memória para `Repository` (para testes e protótipos).
- [ ] Adapter de banco `sqlx` + Postgres, como feature opcional (**D15** ✅).
- [ ] Integração HTTP: generator de endpoint que recebe JSON e dispara um Command (**D15**).
- [ ] `cerne g port <Nome>` e `cerne g adapter <Nome>`.

**Pronto quando:** o `examples/rde` expõe uma API HTTP que persiste transferências num banco.

---

## Fase 4 — Publicação

- [ ] Publicar (nomes livres, D1 ✅) `cerne` e `cerne-cli` (0.1.0).
- [x] Guia "do Event Storming ao código": o README é um tutorial que percorre o `main.rs` do `examples/rde` (D26).
- [ ] Política de versionamento: enquanto estiver em 0.x, mudanças que quebram compatibilidade são permitidas e registradas no CHANGELOG.
- [ ] Definir a MSRV (versão mínima do Rust suportada).
- [x] ~~O `lib.rs` inclui o README da raiz com `include_str!("../../../README.md")`.~~ Resolvido na D26: o `lib.rs` tem a própria documentação. Como o pacote publicado só contém `crates/cerne/`, esse caminho quebra no `cargo publish`. Antes de publicar, copiar o README para dentro da crate (ou apontar `readme` no `Cargo.toml`).

---

## Decisões pendentes

Nenhuma: as 26 decisões estão tomadas. Em aberto dentro da D23: como fica o id gerado pelo repositório. Ver o resumo em [DECISOES.md](./DECISOES.md).
