# Cerne

Framework Rust, no espírito do Rails, em que cada post-it do Event Storming vira código explícito.

## Onde está o contexto

- `docs/ROADMAP.md`: fases, o que está feito ([x]) e o que falta. **Cada fase é feita num chat próprio.**
- `docs/DECISOES.md`: decisões D1–D29, todas tomadas. São a fonte da verdade: siga-as e não reabra uma decisão sem perguntar.
- `README.md`: não é incluído na documentação da crate (o `lib.rs` tem a própria, com a tabela post-it → código). É um tutorial que percorre o `main.rs` do `examples/rde` (D26). Cada bloco marcado com `<!-- snippet: <arquivo> -->` é uma cópia literal do arquivo, e o `examples/rde/tests/readme.rs` falha se a cópia ficar desatualizada. Ao mudar um trecho da rde que aparece no README, atualize o bloco também.

## Estilo de código (o mais importante)

- **Simples e explícito:** quem bate o olho sabe em que parte do Event Storming está. Na dúvida, a versão com menos conceitos vence.
- **Coleção nomeada:** `Invariant`/`Invariants`, `BusinessRule`/`BusinessRules` e `Policy`/`Policies` seguem o mesmo molde. Cada item tem `name: &'static str` + closure, e a coleção tem um método que roda tudo e devolve nomes. Exceção: `Policies::trigger` devolve `FiredPolicy` (nome + command), ver D20.
- **Conceito do usuário = struct própria + trait da lib:** `ValueObject`, `Entity`, `Aggregate`, `DomainEvent<Ports>`, `Command<Ports>` e `Query<Ports>` (que devolve um `ReadModel`). Nada de struct genérica com `name` + `payload`.
- **Variáveis com nome descritivo:** o `impl` lê de cima para baixo, e cada nome diz o que significa no domínio.
  - A condição é calculada antes da closure, numa variável que se lê como frase: `let sender_is_not_recipient = self.sender != self.recipient;` e depois `Invariant::new("...", move || sender_is_not_recipient)`. Nada de `let qty = self.qty;` com a comparação escondida na closure.
  - Cada evento ganha uma variável antes do retorno: `let transfer_created = TransferCreated { .. };` e depois `Ok(vec![Box::new(transfer_created)])`.
  - O resultado de um `execute` vai para uma variável `*_execution`: o `output` vai para quem chamou, e os `events` vão para o processador. Por exemplo, `let create_transfer_execution = create_transfer.execute(&ports).await?;`, depois `let tx_hash = create_transfer_execution.output;` e `processor.send_events(create_transfer_execution.events)`.
- **Sem índice, downcast ou `.expect` para tirar dados de eventos:** o que quem chamou precisa vem no `Output` do command (D25).
- **Seções com linha divisória e muita quebra de linha:** o corpo de um `execute` é dividido nas partes do Event Storming, cada uma aberta por um comentário divisório de 80 colunas (`// --- Business rules ------...`) e separada por linha em branco. A ordem é: `Domain service`, `Ports` (leituras), `Business rules`, `External system: <Nome>`, `Aggregate` (mudança + `save`) e `Domain events` (monta o evento e o `Ok(Executed { .. })`). O mesmo vale para o agregado (`Status`, `Aggregate`, `Entity: identity and invariants`, `State transitions`), para o `trigger_policies` (`Policies`, com uma variável por policy, sempre com o sufixo `_policy`: `chain_accepted_transfer_policy`), para o `main` (um bloco por ator) e para os testes (um bloco por raia do board). Cada `let` de um passo diferente ganha uma linha em branco antes.
- **A variável diz o tipo de processador:** `let outbox_policy_processor = OutboxPolicyProcessor::new(..)` (o padrão), `let sync_policy_processor = InlinePolicyProcessor::new(..)` e `let async_policy_processor = TokioPolicyProcessor::spawn(..)`. Nunca só `processor`.
- **Repositório em memória é SQLite em memória** (`SqliteDatabase::in_memory()`), com o mesmo adapter SQL de produção. Nunca um `Vec` ou `HashMap` fingindo ser repositório (D31).
- **Nomes de método dizem a parte do fluxo:** `send_events`, `send_command`, `trigger_policies`. Um nome vago como `react_to` não serve.
- **Command de criação não recebe id:** quem decide o id é o repositório no insert, ou o próprio conteúdo, quando ele é determinístico (o `tx_hash` do `examples/rde`).
- **Erros:** o domínio devolve `EnforcementResult<T>` (= `Result<T, DomainError>`); commands, repositórios e processadores devolvem `Result<T, cerne::Error>`. A arquitetura com `thiserror`/`anyhow` está em D16.
- **Domínio sync, aplicação async:** `Command`, `Repository` e `PolicyProcessor` usam `#[async_trait]`; entidades, eventos, invariantes, regras e policies nunca são async.
- **Idioma:** código, testes e mensagens em inglês; docs em português.

## Comandos

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

As três precisam passar antes de dar uma fase por concluída.

## Ao terminar uma fase

1. Marcar os itens no `docs/ROADMAP.md` e atualizar a tabela "Onde estamos hoje".
2. Registrar no `docs/DECISOES.md` qualquer decisão nova tomada durante a fase.

## Ao escrever docs

Sujeitos sempre concretos: nada de "todos", "eles", "alguém" quando dá para dizer quem é.
