# Decisões

Cada decisão tem o mesmo formato: **a pergunta**, **por que importa**, **as opções** com prós e contras, **a recomendação** e **o que ela bloqueia** no [ROADMAP](./ROADMAP.md).

Legenda de impacto:
- 🔴 **Difícil de voltar atrás:** muda o código de todos os usuários.
- 🟡 **Média:** dá para mudar, mas custa uma versão que quebra compatibilidade.
- 🟢 **Fácil:** dá para trocar depois sem dor.

---

## Resumo

Todas as decisões, uma linha cada. O texto completo das decisões tomadas (a pergunta, as opções e a discussão) saiu deste arquivo e está no histórico do git: `git show 67cf4c7:docs/DECISOES.md` (D1–D35) e `git show d291f8b:docs/DECISOES.md` (D36). O que foi decidido na D37 está na Fase 5 do [ROADMAP](./ROADMAP.md), e o da D38, da D39 e da D40, na Fase 3.5. Abaixo da tabela, só o texto das decisões em aberto.

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
| D36 | O formato da chamada JSON-RPC | 🔴 | ✅ | `params` por posição (array) e por nome (objeto); `"jsonrpc": "2.0"` conferido (`-32600`); JSON malformado volta `-32700`. Base: a captura da MetaMask ([METAMASK.md](./METAMASK.md)) |
| D37 | O domínio em outras linguagens (TypeScript, Elixir) | 🟡 | ✅ | Vai tudo, menos o que recebe `Ports`; tipos nativos de cada alvo; crate gerada à parte; TypeScript via WebAssembly primeiro, Elixir via Rustler depois |
| D38 | A rde é chamada pela MetaMask | 🔴 | ✅ | `eth_sendRawTransaction` é o `CreateTransferCommand`; valores em wei (`u128`); gas da transação + 1% de descarbonização; um envio em aberto por remetente; rejeitada ou cancelada entra na chain como falha, sem cobrar nada; blocos e recibos no port `Blockchain` |
| D39 | Como a Fase 3.5 foi feita | 🟢 | ✅ | `Request::from_body` na lib; o `-32001` põe as violações no `message`; `cerne g http rest\|jsonrpc` reescreve o `main.rs` só se ele ainda é o do `cerne new`, e em JSON-RPC dá método aos commands de ator e às queries que já existem; o e2e roda o Postgres num banco próprio, criado no `DATABASE_URL`; no SQLite, a transação abre com `BEGIN IMMEDIATE` |
| D40 | O "Cancelar" da MetaMask na rde | 🟡 | ✅ | Valor 0 para a própria conta é cancelamento (`SendCancellationCommand`); "pendente" vale como "em nenhuma proposta de bloco"; o cancelamento entra na chain com o hash novo, `status: "0x1"`, sem taxa, e a transferência fica sem recibo; transação repetida volta com o mesmo hash; "Acelerar" de transferência recusado; chain em SQLite próprio (`rde-chain.db`), apagado com o `rde.db` a cada subida |

---

# Em aberto

Nenhuma decisão em aberto no momento.
