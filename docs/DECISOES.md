# Decisões

Cada decisão tem o mesmo formato: **a pergunta**, **por que importa**, **as opções** com prós e contras, **a recomendação** e **o que ela bloqueia** no [ROADMAP](./ROADMAP.md).

Legenda de impacto:
- 🔴 **Difícil de voltar atrás:** muda o código de todos os usuários.
- 🟡 **Média:** dá para mudar, mas custa uma versão que quebra compatibilidade.
- 🟢 **Fácil:** dá para trocar depois sem dor.

---

## Resumo

Todas as decisões, uma linha cada. O texto completo das decisões tomadas (a pergunta, as opções e a discussão) saiu deste arquivo e está no histórico do git: `git show 67cf4c7:docs/DECISOES.md`. Abaixo da tabela, só o texto das decisões em aberto.

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
| D36 | O formato da chamada JSON-RPC | 🔴 | ⏳ | Em aberto: `params` só por nome (a chamada padrão usa array); `"jsonrpc": "2.0"` não é conferido; JSON malformado não volta `-32700` |
| D37 | O domínio em outras linguagens (TypeScript, Elixir) | 🟡 | ⏳ | Em aberto: fronteira, o que sai, quais alvos |

---

# Em aberto

## D36 — O formato da chamada JSON-RPC 🔴 (em aberto, a partir da D33)

Três problemas na leitura da chamada, todos em `crates/cerne/src/http.rs` e no handler `rpc` (`examples/rde/src/infrastructure/http/rpc.rs` e o `rpc.rs.jinja` do CLI). O primeiro pede uma decisão; os outros dois são erros a corrigir.

### 1. `params` por nome ou por posição

**Problema:** a chamada JSON-RPC que o Cerne documenta e testa manda os `params` como **objeto**, com os campos do command por nome:

```json
{ "jsonrpc": "2.0", "method": "create_transfer", "params": { "sender": "alice", "recipient": "bob", "amount": 100 }, "id": 1 }
```

A forma mais comum de uma chamada JSON-RPC manda os `params` como **array**, por posição:

```json
{ "jsonrpc": "2.0", "method": "create_transfer", "params": ["alice", "bob", 100], "id": 1 }
```

A especificação JSON-RPC 2.0 aceita as duas formas (by-position e by-name). Hoje o `Methods::command` e o `Methods::query` (`crates/cerne/src/http.rs`) leem os `params` com `serde_json::from_value` direto para a struct do command. Os testes (`examples/rde/tests/http.rs`), o `curl` do passo 7 do README e o comentário do `rpc.rs` gerado só usam objeto, então a forma por posição nunca foi testada nem documentada.

**Por que importa:** quem chama a API (uma wallet, um SDK de blockchain, uma ferramenta como o `curl` de um nó Ethereum) espera poder mandar array. E, se a posição valer, a **ordem dos campos do command vira parte da API**: trocar a ordem de dois campos de mesmo tipo quebra os clientes sem erro de compilação.

| Opção | A favor | Contra |
|---|---|---|
| A. Só por posição (array, na ordem dos campos do command) | Igual à chamada padrão | Os nomes somem da chamada; a ordem dos campos vira contrato |
| B. As duas formas, como a especificação | Atende os dois tipos de cliente | A ordem dos campos vira contrato também; mais casos para testar |
| C. Só por nome (como hoje) | Explícito; mudar a ordem dos campos não quebra nada | Foge da chamada padrão; o erro para quem manda array não diz isso |

**Recomendação:** **B**. Provavelmente o `serde` já aceita array numa struct derivada (os campos na ordem da declaração), então a mudança pode ser pequena, mas isso ainda não foi verificado.

**O que muda quando for decidido:**
1. `crates/cerne/src/http.rs`: confirmar (ou implementar) a leitura de `params` em array no `Methods::command` e no `Methods::query`; um `params` que não é array nem objeto volta `-32602`.
2. `examples/rde/tests/http.rs`: um teste com `params` em array para um command e para a query.
3. Documentar que a ordem dos campos do command é contrato: no doc comment do `Methods`, no `rpc.rs.jinja` do CLI e no `command.rs.jinja`.
4. README, passo 7: o `curl` com a forma escolhida.
5. Esta decisão: resposta e status ✅.

### 2. O `"jsonrpc": "2.0"` não é conferido

A struct `Request` (`http.rs`) tem só `method`, `params` e `id`. O `serde` ignora o campo `jsonrpc`, então uma chamada com `"jsonrpc": "1.0"`, ou sem o campo, é atendida como se fosse 2.0. A especificação exige o valor `"2.0"`.

**Correção:** a `Request` ganha o campo `jsonrpc`, e um valor diferente de `"2.0"` (ou a ausência dele) volta `-32600` (invalid request).

### 3. JSON malformado não volta como erro JSON-RPC

O envelope é lido pelo extractor `Json<Request>` do axum, antes de o handler rodar. Um corpo que não é JSON, ou um JSON sem `method`, é recusado pelo axum com um erro HTTP dele (400, 415 ou 422), num corpo que não é uma resposta JSON-RPC. A especificação pede `-32700` (parse error) para JSON malformado e `-32600` (invalid request) para um envelope inválido, sempre com HTTP 200 e uma `Response` com `"id": null`.

**Correção:** o handler recebe o corpo cru (`Bytes` ou `String`) e o lê com `serde_json`; a lib dá uma função que transforma o corpo numa `Request` ou num `ErrorObject` com `-32700`/`-32600`. O `rpc.rs` da rde e o `rpc.rs.jinja` do CLI passam a usá-la.

**Testes que faltam (`examples/rde/tests/http.rs`):** corpo que não é JSON → `-32700`; JSON sem `method` → `-32600`; `"jsonrpc": "1.0"` → `-32600`.

**Resposta:** _pendente_ (item 1). Os itens 2 e 3 são correções, sem escolha a fazer.

---

## D37 — O domínio em outras linguagens 🟡 (em aberto)

**Pergunta:** o domínio de um projeto Cerne (value objects, entidades, invariantes, regras de negócio) pode ser exportado como biblioteca para outras linguagens, como TypeScript e Elixir?

**Por que é viável:** a D9 deixou o domínio síncrono e puro, sem IO. É esse tipo de código que atravessa a fronteira entre linguagens sem dor. Os value objects e os commands já são `serde` (Fase 3).

**O que exporta bem:** value objects com as invariantes (o `TxHash::new` valida igual no front e no Rust), entidades e as transições de estado (`Transfer::new`, `accept`, `chain`), invariantes e regras de negócio. Os erros viram `{:error, ["sender has KYC"]}` no Elixir ou uma exceção com as violações no TypeScript. O domínio do Cerne já é funcional (a transição recebe a entidade e devolve outra), o que combina com Elixir.

**O que não exporta:** a camada de aplicação (commands, repositórios, outbox), que é async e usa `tokio` e `sqlx` (o `sqlx` não roda em WebAssembly), e as policies, que devolvem closures e `Box<dyn Command>`. A outra linguagem implementar os ports e o Rust chamá-los de volta é possível, mas frágil.

**Pontos para decidir:**

1. **A fronteira:**

| Opção | A favor | Contra |
|---|---|---|
| A. JSON (entra JSON, sai JSON ou a lista de violações) | Uma forma só para todos os alvos; o `serde` já faz quase tudo | Sem tipos na outra linguagem, a menos que se gerem à parte |
| B. Tipos nativos de cada alvo (`#[wasm_bindgen]`, structs do Rustler) | Tipos de verdade no TypeScript e no Elixir | Uma tradução por alvo; mais código gerado |

2. **O que sai:** só value objects e entidades (com as transições), ou também as regras de negócio e os eventos?
3. **Os alvos e a ordem:** TypeScript via WebAssembly (`wasm-bindgen` + `wasm-pack`, navegador e Node), Node nativo via `napi-rs`, Elixir via Rustler (NIFs).
4. **Onde mora:** uma crate separada, gerada, que embrulha o domínio (por exemplo, `cerne g bindings --wasm` ou `--elixir`), para o domínio não ganhar atributos de cada alvo.

**Recomendação:** só o domínio, numa crate gerada à parte, com fronteira **A** (JSON); TypeScript via WebAssembly primeiro, Elixir via Rustler depois.

**Resposta:** _pendente_
