# Cerne

Framework Rust, no espírito do Rails, em que cada post-it do Event Storming vira código explícito.

## Onde está o contexto

- **O projeto está pronto (0.1.0).** O `docs/` (ROADMAP, DECISOES, METAMASK) e o `examples/rde` foram apagados depois do commit `58535b1`. As decisões D1–D40 continuam valendo: o texto está em `git show 58535b1:docs/DECISOES.md`, e o planejamento futuro (Fase 5, o domínio em TypeScript e Elixir; Fase 6, skills para agentes) em `git show 58535b1:docs/ROADMAP.md`. Não reabra uma decisão sem perguntar.
- `README.md`: só o nome e os links para as três versões (`README.en.md`, `README.pt-BR.md`, `README.es.md`), enxutas, no molde do README do Rails: o banner, o seletor de idioma centralizado (com bandeiras) e o pitch logo em seguida, sem título; depois o "O que é o Cerne?", as três camadas, as crates, os primeiros passos, contribuição e licença. O `README.en.md` é o `readme` das crates no crates.io.
- `docs/<idioma>/tutorial.md` (`en`, `pt-BR`, `es`): o tutorial, uma loja (`shop`) construída com o CLI. É o conteúdo do site de documentação (ver abaixo). Cada arquivo aparece duas vezes: `<!-- generated: <caminho> -->` (como o CLI o gera) e `<!-- file: <caminho> -->` (preenchido). O `tutorial_builds_and_passes_its_tests` (em `crates/cerne-cli/tests/new_and_generate.rs`) roda os comandos `cerne` do tutorial em inglês, confere os blocos gerados, escreve os preenchidos e roda `cargo clippy` e `cargo test`. Só a prosa, os diagramas e os comentários dos comandos são traduzidos: o `every_tutorial_translation_has_the_same_code` falha se o código divergir. Antes de cada bloco `generated`, a frase cita o comando (ou os comandos) do CLI que gera aquele arquivo, e, na seção 1, cada comando é um bloco `bash` próprio, com um parágrafo de uma linha em cima dizendo o que ele cria. Essas frases não são testadas: confira-as se um comando passar a gerar outros arquivos. A árvore do `tree shop` não é testada: refaça-a se o CLI passar a gerar outros arquivos.
- `examples/shop`: a loja do tutorial, gerada com os comandos `cerne` do `docs/en/tutorial.md` e preenchida com os blocos `<!-- file -->`. É membro do workspace e depende do `crates/cerne` por `path`, para mostrar como cada mudança (as macros, por exemplo) muda o código gerado. `python3 examples/regenerate-shop.py` apaga e gera a loja de novo, com o CLI deste checkout.
- Site de documentação (`cerne.ecdesa.com.br`): um projeto à parte, em `~/cerne.ecdesa.com.br` (ver "Site de documentação" abaixo).
- `assets/`: o logo (anéis partidos em seções com a Estrela de Ishtar no miolo, nas cores da ECDESA). Toda imagem tem versão `-dark` e `-light`. `cerne-banner-{dark,light}.svg` (ícone + nome sobre fundo próprio) fica no topo dos READMEs em dois `<img>`, com `#gh-light-mode-only` e `#gh-dark-mode-only` no fim do `src`: assim o GitHub segue o tema escolhido nas configurações dele, não só o do sistema (o `<picture>` só segue o do sistema). O favicon do docs.rs vem do `#![doc(html_favicon_url = ..)]` em `crates/cerne/src/lib.rs`, que aponta para o `cerne-favicon.svg` no raw.githubusercontent da `main`. Também há `cerne-logo-{dark,light}.svg` (ícone + nome, sem fundo), `cerne-icon-{dark,light}.svg`, `cerne-avatar-{dark,light}.{svg,png}` (512px, para o GitHub e o crates.io) e `cerne-favicon.svg` (dois anéis; um arquivo só, que troca de cor sozinho com o tema). O nome "cerne" é desenhado com traços, sem depender de fonte.
- `CHANGELOG.md`: toda quebra de compatibilidade. `CONTRIBUTING.md`: o estilo de código, em inglês, para quem contribui.
- MSRV: Rust 1.88 (`rust-version` no workspace, job `msrv` no CI).

## Site de documentação

`cerne.ecdesa.com.br` é um projeto à parte, fora deste repositório, em `~/cerne.ecdesa.com.br` (ainda sem git). O v0.dev gerou o projeto; o Claude o refina e o mantém.

- **Stack:** Next.js 16 com export estático (`output: "export"`, saída em `out/`), para o Cloudflare Pages. Tailwind 4, `react-markdown` + `remark-gfm`, Shiki no build e Mermaid no cliente. Identidade visual da ECDESA (dark-first, dourado `#E8B923`, navy `#151A5C`, Inter, Space Grotesk e JetBrains Mono); as cores de post-it são livres.
- **Rodar:** o pnpm não está instalado na máquina; use `corepack pnpm` (o `packageManager` do `package.json` fixa a versão). Na pasta do site:

  ```bash
  CERNE_LOCAL_REPO=/home/lucy/cerne corepack pnpm dev --port 3100
  corepack pnpm build
  npx tsc --noEmit
  ```

  Sem `CERNE_LOCAL_REPO`, o site baixa o conteúdo do `main` no GitHub; com ela, lê os arquivos deste checkout, o que mostra uma mudança antes do push. O Next.js não percebe mudanças nesses arquivos: recarregue a página no navegador depois de editar o tutorial ou um README.
- **Conteúdo:** nenhum texto do Cerne é copiado para o site. No build, `lib/site-content.ts` lê os três `docs/<idioma>/tutorial.md`, os três READMEs e o logo de `assets/`. Só os textos da interface ficam no site, em `content/<idioma>/ui.ts`. Por isso o site depende da ordem das seções dos READMEs (pitch, "What's Cerne?", "Crates", "Getting Started", "License") e dos marcadores `<!-- generated -->`/`<!-- file -->` do tutorial: avise antes de mudar um dos dois.
- **Rotas:** `/` e `/tutorial` em inglês; `/pt-BR`, `/pt-BR/tutorial`, `/es` e `/es/tutorial`.
- **Onde fica cada coisa:** `components/markdown-renderer.tsx` transforma o markdown do tutorial (blocos `bash`, `console`, `mermaid` e os pares `generated`/`file`); `components/code-file.tsx` mostra cada par: o código gerado sem as linhas `use` e, embaixo, só as linhas que mudam ao preencher (o diff está em `lib/code-diff.ts`); `components/landing-page.tsx` monta a landing a partir dos READMEs.
- **Deploy:** ainda não publicado. As pendências estão no `TODO.md`.

## Estilo de código (o mais importante)

- **Formatação:** o `rustfmt.toml` (no repositório e em todo projeto do `cerne new`) dá 120 colunas, com os argumentos de uma chamada numa linha só enquanto couberem (`use_small_heuristics = "Max"`). Uma lista passa a um item por linha acima de 80 colunas (`array_width = 80`): duas invariantes quebram, uma sozinha fica numa linha. Os struct literals (`struct_lit_width = 18`) e as cadeias de métodos (`chain_width = 60`) continuam quebrando como no padrão: o CLI acha onde inserir código por linhas como `outbox: Box::new(` e `.with_state(ports)`.
- **Simples e explícito:** quem bate o olho sabe em que parte do Event Storming está. Na dúvida, a versão com menos conceitos vence.
- **Coleção nomeada:** `Invariant`/`Invariants`, `BusinessRule`/`BusinessRules` e `Policy`/`Policies` seguem o mesmo molde. Cada item tem `name: &'static str` + closure, e a coleção tem um método que roda tudo e devolve nomes. Invariantes e regras de negócio são escritas com `invariant!("nome", condição)` e `business_rule!(..)` (a macro escreve o `move ||`) e rodam com `Invariants::enforce([..])?` e `BusinessRules::check([..])?`, que recebem qualquer `IntoIterator` (um array ou um `Vec`). As policies são escritas com `policy!("nome", condição, Command { .. })`, sempre com três argumentos (`true` numa policy que sempre dispara), e disparadas com `Policies::trigger([..])`. O command só é montado se a policy dispara, e leva os valores que usa (o `then` é `FnOnce`): nada de `.clone()` dentro dele. Exceção: `Policies::trigger` devolve `FiredPolicy` (nome + command), ver D20.
- **Entity e aggregate pelos atributos:** `#[entity]`/`#[aggregate]` (crate `cerne-macros`, reexportada em `cerne::domain`) escrevem o `impl Entity`, o `<Nome>Constructor` (todos os campos menos o `id` e os marcados com `#[skip_constructor]`, que começam no `Default`) e, no `#[aggregate]`, o `impl Aggregate`. As invariantes ficam num `impl Validate` escrito à mão, que o CLI gera vazio. Um `Entity` à mão só para id que nunca é `None` (um hash, um documento). O `#[value_object]`, num value object de um campo só (`OrderId(u64)`), escreve o `TryFrom`/`From` do valor e, se a struct deriva `Serialize`/`Deserialize`, o `#[serde(try_from, into)]`; o `impl ValueObject`, com as invariantes, é escrito à mão.
- **Conceito do usuário = struct própria + trait da lib:** `ValueObject`, `Entity`, `Aggregate`, `DomainEvent<Ports>`, `Command<Ports>` e `Query<Ports>` (que devolve um `ReadModel`). Nada de struct genérica com `name` + `payload`.
- **Variáveis com nome descritivo:** o `impl` lê de cima para baixo, e cada nome diz o que significa no domínio.
  - A condição é calculada antes, numa variável que se lê como frase: `let sender_is_not_recipient = self.sender != self.recipient;` e depois `business_rule!("...", sender_is_not_recipient)`. Nada de comparação escrita direto dentro da macro.
  - Cada evento ganha uma variável antes do retorno: `let transfer_created = TransferCreated { .. };` e depois `Ok(vec![Box::new(transfer_created)])`.
  - Numa chamada com dois argumentos do mesmo tipo, cada argumento é uma variável com o mesmo nome do parâmetro: `transfer(sender, recipient, amount)`, com `sender` e `recipient` do mesmo tipo. Trocar os dois não dá erro de compilação, mas fica visível na leitura. Nada de literal ou expressão no lugar, e nada de struct só para nomear os argumentos.
  - O resultado de um `execute` vai para uma variável `*_execution`: o `output` vai para quem chamou, e os `events` vão para o processador. Por exemplo, `let create_transfer_execution = create_transfer.execute(&ports).await?;`, depois `let tx_hash = create_transfer_execution.output;` e `processor.send_events(create_transfer_execution.events)`.
- **Sem índice, downcast ou `.expect` para tirar dados de eventos:** o que quem chamou precisa vem no `Output` do command (D25).
- **Seções com linha divisória e muita quebra de linha:** o corpo de um `execute` é dividido nas partes do Event Storming, cada uma aberta por um comentário divisório de 80 colunas (`// --- Business rules ------...`) e separada por linha em branco. A ordem é: `Domain service`, `Ports` (leituras), `Business rules`, `External system: <Nome>`, `Aggregate` (mudança + `save`) e `Domain events` (monta o evento e o `Ok(Executed { .. })`). O mesmo vale para o agregado (`Status`, `Aggregate`, `Invariants`, `State transitions`), para o `trigger_policies` (`Policies`, com uma variável por policy, sempre com o sufixo `_policy`: `charge_the_customer_policy`), para o `main` (um bloco por ator) e para os testes (um bloco por fluxo do board). Cada `let` de um passo diferente ganha uma linha em branco antes.
- **Regras de negócio no `execute`:** na seção `Business rules`, cada condição numa variável e depois o `BusinessRules::check([..])?`. Só um projeto que exporta o domínio para outras linguagens (D37, Fase 5) tira as regras do `execute` para um método síncrono do command, `pub fn business_rules(&self, ..) -> Vec<BusinessRule>`, que recebe o que a seção `Ports` leu (ou o agregado carregado); o `execute` passa a chamar `BusinessRules::check(self.business_rules(..))?`. Numa aplicação só em Rust, o método não traz nada.
- **A variável diz o tipo de processador:** `let outbox_policy_processor = OutboxPolicyProcessor::new(..)` (o padrão), `let sync_policy_processor = InlinePolicyProcessor::new(..)` e `let async_policy_processor = TokioPolicyProcessor::spawn(..)`. Nunca só `processor`.
- **Repositório em memória é SQLite em memória** (`SqliteDatabase::in_memory()`), com o mesmo adapter SQL de produção. Nunca um `Vec` ou `HashMap` fingindo ser repositório (D31).
- **Nomes de método dizem a parte do fluxo:** `send_events`, `send_command`, `trigger_policies`. Um nome vago como `react_to` não serve.
- **Command de criação não recebe id:** quem decide o id é o repositório no insert, ou o próprio conteúdo, quando ele é determinístico (o hash de uma transação assinada).
- **Erros:** o domínio devolve `EnforcementResult<T>` (= `Result<T, DomainError>`); commands, repositórios e processadores devolvem `Result<T, cerne::Error>`. A arquitetura com `thiserror`/`anyhow` está em D16.
- **Domínio sync, aplicação async:** `Command`, `Repository` e `PolicyProcessor` usam `#[async_trait]`; entidades, eventos, invariantes, regras e policies nunca são async.
- **Idioma:** código, testes e mensagens em inglês; o README em inglês, português e espanhol; o rustdoc e o CHANGELOG em inglês.

## Comandos

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

As três precisam passar antes de dar uma fase por concluída.

## Ao mudar a API

1. Registrar no `CHANGELOG.md` o que quebra compatibilidade.
2. Atualizar o tutorial nos três READMEs, se ele usa o que mudou.

## Ao escrever docs

Sujeitos sempre concretos: nada de "todos", "eles", "alguém" quando dá para dizer quem é.
