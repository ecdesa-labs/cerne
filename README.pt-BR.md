<p align="center">
  <img alt="Cerne" src="assets/cerne-banner-light.svg#gh-light-mode-only" width="400">
  <img alt="Cerne" src="assets/cerne-banner-dark.svg#gh-dark-mode-only" width="400">
</p>

<p align="center">
  🇺🇸 <a href="README.en.md">English</a> ·
  🇧🇷 Português ·
  🇪🇸 <a href="README.es.md">Español</a>
</p>

<br>

Um sistema não vira legado porque alguém o escreveu mal. Ele vira legado porque, ao longo dos anos, recebe milhares de mudanças que fazem sentido uma a uma: um acordo sindical, uma lei nova, uma decisão judicial, um imposto que mudou. Cada equipe resolve o seu ticket do jeito que dá, muitas vezes de madrugada, com um `if` no meio do código "só por enquanto". A regra antiga continua rodando ao lado da nova, porque ninguém tem coragem de mexer. Como resume o canal gotoCobol:

> "A entropia nasce do acúmulo de soluções localmente justificáveis, mas que nunca são reorganizadas como um todo."

Sistemas assim são construídos todos os dias, e a conta chega anos depois, na hora de modernizar. Aí não basta trocar de linguagem: as regras de negócio estão espalhadas, sem nome e sem história, e é preciso desenterrar cada uma antes de mudar qualquer coisa.

O Cerne não impede o sistema de mudar. Ele dá a cada mudança um lugar certo e um nome:

- **Toda regra tem nome e endereço.** Uma decisão judicial vira uma `BusinessRule` com a frase dela, na seção `Business rules` do command que ela afeta, e não um `if` perdido no meio do código. Quem chega depois acha a regra pelo nome, e quando ela recusa um pedido, o erro diz qual foi.
- **O código tem a forma do board.** Cada post-it do Event Storming é um tipo Rust, e todo `execute` tem as mesmas seções, na mesma ordem. Equipes diferentes, em anos diferentes, podem escrever no mesmo formato, porque o `cerne g` dá a todas o mesmo esqueleto.
- **O board e o código contam a mesma história.** A conversa com o negócio acontece no board, com as mesmas palavras do código. Uma regra nova começa como um post-it e termina no lugar que o post-it indica.
- **Uma reação é uma policy, não um efeito colateral.** "Sempre que X, faça Y" vira uma `Policy` com nome, e o event bus executa o command dela. Ninguém precisa caçar onde a reação foi parar.

Essa é a proposta do Cerne: um sistema que dure 30 anos e continue legível, não porque nada mudou, mas porque cada mudança ficou à vista.

Inspirado no vídeo [Entropia de software não é a mesma coisa que complexidade estrutural](https://www.youtube.com/watch?v=wTQbXp86m78), do canal gotoCobol.

## O que é o Cerne?

O Cerne é um framework Rust que transforma um board de [Event Storming](https://www.eventstorming.com) em código: cada post-it vira um tipo seu que implementa uma trait do Cerne, e cada fluxo vira um `execute` com as seções do board.

Para a aplicação seguir essa forma enquanto cresce, o Cerne vem com um CLI, o `cerne`. O `cerne new` cria o projeto já organizado como o board, e cada `cerne g` gera a peça nova (um command, um evento, uma entidade, um port) no lugar certo, com as seções certas e já compilando. Quem chega ao projeto não precisa adivinhar onde a feature nova mora nem como escrevê-la: ela já nasce dentro da proposta.

Entender o Event Storming é a chave para entender o Cerne. O board conta uma história com post-its coloridos: um ator envia um command (azul), um agregado (amarelo) muda, um evento de domínio (laranja) acontece, uma policy (lilás) reage com um novo command, sistemas externos (rosa) são chamados, e read models (verde) mostram o resultado. O Cerne divide a aplicação em três camadas: Domain, Application e Infrastructure.

### Camada Domain

A camada Domain é o coração do board: entidades e agregados (`Entity`, `Aggregate`), value objects (`ValueObject`), eventos de domínio (`DomainEvent`) e as policies que reagem a eles (`Policy`). Ela é pura e síncrona: nenhum IO acontece aqui. As invariantes (`Invariant`) são regras com nome sobre o que é sempre verdade, e quando uma quebra, o erro lista os nomes como estão escritos no board: `["quantity is positive"]`.

### Camada Application

A camada Application é onde os atores agem. Um command (`Command`) abre a sua transação, lê os ports, confere as regras de negócio (`BusinessRule`), muda um agregado e grava os eventos no event outbox (`EventOutbox`), sempre nessa ordem, então o `execute` se lê como um fluxo do board. Uma query (`Query`) devolve um read model, uma struct de campos simples. Os ports são traits assíncronas para os repositórios e os sistemas externos. Quem enviou o command publica os eventos dele num event bus, que executa as policies: o `SyncEventBus`, uma cadeia por vez, ou o `AsyncEventBus`, todos os eventos ao mesmo tempo. O bus torce pelo melhor: uma policy que falha vai para o `on_error` dele, e como cada policy sobrevive a uma falha é decisão da aplicação.

### Camada Infrastructure

O Cerne é, antes de tudo, um framework de modelagem de domínio e de aplicação, não de infraestrutura, e não traz adapters. Os ports são traits: os repositórios, o event outbox e cada sistema externo são implementados pela aplicação, sobre o banco, o framework web e a fila que ela escolher. O `cerne new` escreve uma função com um `todo!()` onde entra cada adapter.

## Crates

- [`cerne`](https://crates.io/crates/cerne): a biblioteca.
- [`cerne-cli`](https://crates.io/crates/cerne-cli): o comando `cerne`, que cria um projeto organizado como o board (`cerne new`) e gera cada post-it no seu lugar, já compilando (`cerne g`).
- [`cerne-macros`](https://crates.io/crates/cerne-macros): os atributos `#[entity]`, `#[aggregate]` e `#[value_object]`. O `cerne::domain` os reexporta, então um projeto só depende do `cerne`.

## Primeiros passos

1. Instale o comando `cerne` (Rust 1.88 ou mais novo):

   ```bash
   cargo install cerne-cli
   ```

2. Crie um projeto:

   ```bash
   cerne new shop
   ```

3. Gere um agregado e um command, e rode os testes:

   ```bash
   cd shop
   cerne g entity Order product:String quantity:u32 --aggregate
   cerne g command PlaceOrder product:String quantity:u32
   cargo test
   ```

4. Preencha os post-its. Estes recursos ajudam:
   - [O tutorial](docs/pt-BR/tutorial.md): uma loja, do board ao código, com todos os post-its.
   - [A documentação da API](https://docs.rs/cerne)
   - O `cerne` sem argumentos lista todos os generators.

## Contribuindo

Contribuições são bem-vindas. O [CONTRIBUTING.md](CONTRIBUTING.md) explica como rodar as verificações e o estilo de código. Toda mudança que quebra compatibilidade vai para o [CHANGELOG](CHANGELOG.md).

## Licença

O Cerne é distribuído sob a [Apache License 2.0](LICENSE). O código que o `cerne new` e o `cerne g` geram pertence a quem o gerou, que pode usá-lo sob qualquer licença.
