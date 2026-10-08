# Bem-vindo ao Cerne

[English](README.en.md) · Português · [Español](README.es.md)

## Por que o Cerne?

Um sistema não vira legado porque alguém o escreveu mal. Ele vira legado porque, ao longo dos anos, recebe milhares de mudanças que fazem sentido uma a uma: um acordo sindical, uma lei nova, uma decisão judicial, um imposto que mudou. Cada equipe resolve o seu ticket do jeito que dá, muitas vezes de madrugada, com um `if` no meio do código "só por enquanto". A regra antiga continua rodando ao lado da nova, porque ninguém tem coragem de mexer. Como resume o canal gotoCobol:

> "A entropia nasce do acúmulo de soluções localmente justificáveis, mas que nunca são reorganizadas como um todo."

Sistemas assim são construídos todos os dias, e a conta chega anos depois, na hora de modernizar. Aí não basta trocar de linguagem: as regras de negócio estão espalhadas, sem nome e sem história, e é preciso desenterrar cada uma antes de mudar qualquer coisa.

O Cerne não impede o sistema de mudar. Ele dá a cada mudança um lugar certo e um nome:

- **Toda regra tem nome e endereço.** Uma decisão judicial vira uma `BusinessRule` com a frase dela, na seção `Business rules` do command que ela afeta, e não um `if` perdido no meio do código. Quem chega depois acha a regra pelo nome, e quando ela recusa um pedido, o erro diz qual foi.
- **O código tem a forma do board.** Cada post-it do Event Storming é um tipo Rust, e todo `execute` tem as mesmas seções, na mesma ordem. Equipes diferentes, em anos diferentes, podem escrever no mesmo formato, porque o `cerne g` dá a todas o mesmo esqueleto.
- **O board e o código contam a mesma história.** A conversa com o negócio acontece no board, com as mesmas palavras do código. Uma regra nova começa como um post-it e termina no lugar que o post-it indica.
- **Uma reação é uma policy, não um efeito colateral.** "Sempre que X, faça Y" vira uma `Policy` com nome, e o command dela passa pela outbox. Ninguém precisa caçar onde a reação foi parar.

Essa é a proposta do Cerne: um sistema que dure 30 anos e continue legível, não porque nada mudou, mas porque cada mudança ficou à vista.

Inspirado no vídeo [Entropia de software não é a mesma coisa que complexidade estrutural](https://www.youtube.com/watch?v=wTQbXp86m78), do canal gotoCobol.

## O que é o Cerne?

<!-- pitch: provisório -->
O Cerne é um framework Rust que transforma um board de [Event Storming](https://www.eventstorming.com) em código. Cada post-it do board vira um bloco explícito de código, e quem abre um arquivo sabe na hora em que parte do fluxo está.

Para a aplicação seguir essa forma enquanto cresce, o Cerne vem com um CLI, o `cerne`. O `cerne new` cria o projeto já organizado como o board, e cada `cerne g` gera a peça nova (um command, um evento, uma entidade, um port) no lugar certo, com as seções certas e já compilando. Quem chega ao projeto não precisa adivinhar onde a feature nova mora nem como escrevê-la: ela já nasce dentro da proposta.

Entender o Event Storming é a chave para entender o Cerne. O board conta uma história com post-its coloridos: um ator envia um command (azul), um agregado (amarelo) muda, um evento de domínio (laranja) acontece, uma policy (lilás) reage com um novo command, sistemas externos (rosa) são chamados, e read models (verde) mostram o resultado. O Cerne dá a cada post-it uma trait Rust e divide a aplicação em três camadas: Domain, Application e Infrastructure.

### Camada Domain

A camada Domain é o coração do board: entidades e agregados (`Entity`, `Aggregate`), value objects (`ValueObject`), eventos de domínio (`DomainEvent`) e as policies que reagem a eles (`Policy`). Ela é pura e síncrona: nenhum IO acontece aqui. As invariantes (`Invariant`) são regras com nome sobre o que é sempre verdade, e quando uma quebra, o erro lista os nomes como estão escritos no board: `["quantity is positive"]`.

### Camada Application

A camada Application é onde os atores agem. Um command (`Command`) lê os ports, confere as regras de negócio (`BusinessRule`), muda um agregado e devolve os eventos, sempre nessa ordem, então o `execute` se lê como um fluxo do board. Uma query (`Query`) devolve um read model (`ReadModel`). Os ports são traits assíncronas para os repositórios e os sistemas externos. Os commands que as policies disparam vão para uma outbox na mesma transação do agregado, e o `OutboxPolicyProcessor` os executa, mesmo que o processo caia no meio.

### Camada Infrastructure (opcional)

O Cerne é, antes de tudo, um framework de modelagem de domínio e de aplicação, não de infraestrutura. A camada Infrastructure é um extra para acelerar o desenvolvimento: adapters prontos para os repositórios SQL (`cerne::sqlite`, também em memória, e `cerne::postgres`, com a mesma API e o mesmo SQL) e para o HTTP, em REST ou JSON-RPC 2.0 (feature `axum`).

Nada nas camadas Domain e Application depende desses adapters. Os ports são traits, e qualquer adapter que as implemente serve: outro banco, outro framework web, uma fila. Para usar o Cerne sem nenhum adapter dele:

```toml
cerne = { version = "0.1", default-features = false }
```

## Crates

- [`cerne`](https://crates.io/crates/cerne): a biblioteca. As features `sqlite` (padrão), `postgres` e `axum` são a camada Infrastructure, opcional.
- [`cerne-cli`](https://crates.io/crates/cerne-cli): o comando `cerne`, que cria um projeto organizado como o board (`cerne new`) e gera cada post-it no seu lugar, já compilando (`cerne g`).

## Primeiros passos

1. Instale o comando `cerne` (Rust 1.88 ou mais novo):

   ```bash
   cargo install cerne-cli
   ```

2. Crie um projeto com uma API REST:

   ```bash
   cerne new shop --http rest
   ```

3. Gere um command e a rota dele, e suba o servidor:

   ```bash
   cd shop
   cerne g command PlaceOrder product:String quantity:u32
   cerne g endpoint PlaceOrder POST /orders
   cargo run
   ```

4. Envie o command:

   ```console
   $ curl -X POST localhost:3000/orders -H 'content-type: application/json' -d '{"product": "mug", "quantity": 2}'
   null
   ```

5. Preencha os post-its. Estes recursos ajudam:
   - [O tutorial](docs/pt-BR/tutorial.md): uma loja, do board ao código, com todos os post-its.
   - [A documentação da API](https://docs.rs/cerne)
   - O `cerne` sem argumentos lista todos os generators.

## Contribuindo

Contribuições são bem-vindas. O [CONTRIBUTING.md](CONTRIBUTING.md) explica como rodar as verificações e o estilo de código. Toda mudança que quebra compatibilidade vai para o [CHANGELOG](CHANGELOG.md).

## Licença

O Cerne é distribuído sob a [Apache License 2.0](LICENSE). O código que o `cerne new` e o `cerne g` geram pertence a quem o gerou, que pode usá-lo sob qualquer licença.
