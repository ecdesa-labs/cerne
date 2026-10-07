# Licenças: qual escolher para o Cerne

> Guia para decidir a **D3**. Explica o que cada licença permite, o que exige em troca e em que situação essas exigências passam a valer. Não substitui um advogado, mas é suficiente para uma decisão informada.

---

## Resumo em 30 segundos

- **Recomendação:** `MIT OR Apache-2.0`, o mesmo padrão do próprio Rust.
- **Para quem usa o Cerne:** pode criar qualquer aplicação, fechada ou aberta, sem pedir permissão, e não precisa publicar o próprio código.
- **Para você e quem contribui:** quem usa o Cerne pela Apache-2.0 recebe uma licença das patentes dos contribuidores. Essa pessoa ou empresa perde a licença se processar alguém alegando que o Cerne viola uma patente.
- **O que fica em aberto:** escolher uma licença copyleft só faz sentido se você quiser **obrigar** a devolução de melhorias, ou planejar vender o Cerne no futuro.

---

## Conceitos que você precisa antes de comparar

### 1. Permissiva × copyleft

| Tipo | Ideia | Exemplos |
|---|---|---|
| **Permissiva** | "Use como quiser, só mantenha os créditos." Quem usa pode fechar o código derivado. | MIT, Apache-2.0 |
| **Copyleft** | "Se você distribuir algo feito com isto, distribua também o código-fonte, sob esta mesma licença." | MPL-2.0 (fraca), GPL-3.0 (forte), AGPL-3.0 (forte + rede) |

### 2. O que "aciona" as obrigações

Quase todas as obrigações só valem quando alguém **distribui** o software, ou seja, entrega uma cópia a outra pessoa: um binário para download, um app instalado no cliente, uma imagem Docker vendida.

**Rodar o software no próprio servidor e oferecer uma API não é distribuir.** É por isso que a GPL não obriga empresas de SaaS a abrir o código, e foi para fechar essa brecha que a AGPL foi criada.

Isso é decisivo para o Cerne, porque ele serve para construir APIs que rodam em servidor.

### 3. Patentes

O direito autoral protege o **texto** do código. A patente protege uma **ideia ou técnica**, independentemente de como foi escrita. Uma empresa pode ter a patente de uma técnica que algum contribuidor implementou no Cerne sem saber.

Uma licença que fala de patentes responde a duas perguntas:
- Quem contribuiu código promete não processar os usuários por causa das próprias patentes?
- O que acontece com quem abre um processo de patente contra o projeto?

A MIT não responde a nenhuma das duas. A Apache-2.0 responde às duas.

### 4. Marca

Licença de código não é licença de marca. Mesmo com o código livre, o **nome "Cerne"** pode continuar protegido. A Apache-2.0 diz isso explicitamente; a MIT não diz nada.

---

## MIT

**Em uma frase:** qualquer pessoa ou empresa que **obtiver uma cópia** do código pode fazer o que quiser com ele, desde que mantenha o aviso de copyright e o texto da licença.

**Quem dá a permissão:** o titular do copyright, isto é, você e cada contribuidor sobre o código que escreveu.

**Permite:** usar, copiar, modificar, juntar com outro código, publicar, distribuir, sublicenciar e vender.

**Exige:** incluir o aviso de copyright e o texto da licença em todas as cópias ou partes substanciais do código.

**Não oferece:**
- **Patentes:** não há concessão explícita. Juristas discutem se existe uma licença implícita, mas não há garantia escrita.
- **Garantia:** o software é fornecido "como está", e o autor não responde por danos.

**Por que tanta gente usa:** o texto é curto (cerca de 170 palavras), qualquer pessoa entende, e o departamento jurídico de qualquer empresa aprova sem discussão.

**Quem usa:** Ruby on Rails, `tokio`, `axum`, React.

---

## Apache-2.0

**Em uma frase:** as mesmas liberdades da MIT, mais regras claras para patentes, marcas, contribuições e arquivos modificados.

O texto é longo porque cobre situações que a MIT deixa em silêncio. Antes das seções, é preciso saber quem é quem, porque a licença inteira é escrita em função desses três papéis.

### Seção 1: quem é quem

A Apache-2.0 define cada parte envolvida. Aplicado ao Cerne:

| Termo na licença | Definição da licença, resumida | No Cerne |
|---|---|---|
| **Licenciante** (*Licensor*) | O titular do copyright, ou quem ele autorizou, que está concedendo a licença | Você, que criou o Cerne |
| **Contribuidor** (*Contributor*) | O Licenciante e qualquer pessoa ou empresa cuja contribuição foi recebida e incorporada ao projeto | Você + cada pessoa ou empresa que teve um PR aceito |
| **Você** (*You*) | A pessoa ou empresa que exerce as permissões da licença | Quem baixa, usa, modifica ou redistribui o Cerne, inclusive quem o recebe como dependência de outro projeto |

Então, quando a licença diz que "cada Contribuidor concede a Você", isso significa: **cada pessoa ou empresa que escreveu código do Cerne concede a cada pessoa ou empresa que usa o Cerne**.

Um mesmo agente pode ter dois papéis. Uma empresa que envia um PR e também usa o Cerne é Contribuidor e é "Você" ao mesmo tempo.

### Seção 2: direito autoral

Cada Contribuidor concede a cada pessoa ou empresa que usa o Cerne ("Você") uma licença **perpétua, mundial, gratuita e irrevogável** para reproduzir, modificar, exibir, sublicenciar e distribuir o código.

É o equivalente à MIT, só que dito com todas as letras: "perpétua" e "irrevogável" significam que nem você nem nenhum Contribuidor pode retirar essa permissão de quem já recebeu o código.

### Seção 3: patentes (a principal diferença)

**A concessão.** Cada Contribuidor concede a quem usa o Cerne uma licença para as patentes **dele** que seriam infringidas pelo código que **ele** contribuiu, sozinho ou combinado com o restante do projeto.

> **Exemplo:** a Empresa X envia ao Cerne um PR que implementa uma técnica de serialização patenteada por ela. Ao contribuir, a Empresa X concede automaticamente a cada pessoa ou empresa que usa o Cerne o direito de usar essa patente **naquele código**. Ela não pode depois processar quem usa o Cerne por causa dessa técnica.

Essa concessão não cobre todas as patentes do Contribuidor, apenas as que o código que ele enviou necessariamente usa.

**A cláusula de retaliação.** Se uma pessoa ou empresa que usa o Cerne processar **qualquer outra pessoa ou empresa** alegando que o Cerne (ou uma contribuição a ele) viola uma patente, **todas as licenças de patente que ela recebeu pela Apache-2.0 terminam** na data do processo.

> **Exemplo:** a Empresa Y usa o Cerne e, ao mesmo tempo, processa a fintech Z, que também usa o Cerne, dizendo que o Cerne viola uma patente dela. A partir desse dia, a Empresa Y perde as licenças de patente que recebeu de cada Contribuidor do Cerne, inclusive a da Empresa X do exemplo anterior, e passa a poder ser processada por elas.

O direito autoral (seção 2) **não** é revogado, só as patentes. É um desestímulo forte: quem ataca o projeto perde a proteção que ele oferece.

### Seção 4: redistribuição

Quem distribuir o Cerne, original ou modificado, precisa:

1. **Entregar uma cópia da licença** junto.
2. **Marcar os arquivos modificados** com um aviso visível dizendo que foram alterados. Assim quem recebe o fork sabe que aquele arquivo não é mais o original, e um bug introduzido no fork não é atribuído a você.
3. **Manter os avisos** de copyright, patente, marca e atribuição que já estavam no código.
4. **Repassar o arquivo `NOTICE`**, se o projeto tiver um. É um arquivo opcional de créditos, e o conteúdo dele precisa aparecer em algum lugar visível do produto derivado (documentação, tela "Sobre" etc.).

Quem modifica pode colocar **as próprias modificações** sob outra licença, inclusive fechada. O código original continua Apache-2.0.

### Seção 5: contribuições

Qualquer código enviado de propósito para o projeto (um PR, por exemplo) fica automaticamente sob a Apache-2.0, a menos que o contribuidor diga o contrário.

Isso dispensa um contrato de contribuição (CLA) para a maioria dos projetos. Quem envia um PR já concordou com os termos, inclusive com a concessão de patentes da seção 3.

### Seção 6: marca

A licença **não** dá direito de usar o nome, o logotipo ou as marcas do projeto, exceto para descrever de onde o código veio.

> **Exemplo:** alguém pode fazer um fork e escrever "baseado no Cerne", mas não pode chamar o fork de "Cerne Pro" e dar a entender que é o produto oficial.

### Seções 7 a 9: garantia e responsabilidade

- **Seções 7 e 8:** o código é fornecido "como está", e os autores não respondem por danos (como na MIT).
- **Seção 9:** quem redistribui pode **oferecer garantia ou suporte pago**, mas só em nome próprio, sem envolver os autores originais. Isso abre espaço para empresas venderem suporte ao Cerne.

### Compatibilidade com a GPL

- **GPL-3.0:** compatível. Código Apache-2.0 pode ser incluído num projeto GPL-3.0.
- **GPL-2.0:** **incompatível**, segundo a Free Software Foundation. A cláusula de retaliação de patentes é uma restrição que a GPL-2.0 não permite. É por isso que o Rust oferece a MIT como alternativa.

**Quem usa:** Kubernetes, Android (AOSP), Swift, TensorFlow.

---

## MIT OR Apache-2.0 (licença dupla)

**Em uma frase:** o projeto é publicado sob as duas licenças, e **cada usuário escolhe** qual seguir.

Cada uma cobre uma lacuna da outra:

| Quem usa o Cerne | Escolhe | Por quê |
|---|---|---|
| Uma empresa preocupada com patentes | Apache-2.0 | Recebe a concessão de patentes dos contribuidores |
| Um projeto GPL-2.0 (ex.: algo ligado ao kernel Linux) | MIT | A Apache-2.0 seria incompatível |
| Qualquer outra pessoa | Tanto faz | As duas permitem tudo o que ela precisa |

**Contribuições** entram sob as duas licenças: quem envia um PR o licencia pela MIT **e** pela Apache-2.0, e cada usuário escolhe qual das duas seguir. No Rust isso é feito com uma frase padrão no README:

> Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.

**Custo:** dois arquivos de licença na raiz (`LICENSE-MIT` e `LICENSE-APACHE`) e essa frase no README.

**Quem usa:** o compilador e a biblioteca padrão do Rust, `serde`, `sqlx`, `actix-web` e a maior parte das crates populares.

---

## MPL-2.0 (copyleft por arquivo)

**Em uma frase:** quem modificar **arquivos do Cerne** e distribuir o resultado precisa publicar esses arquivos modificados; o resto da aplicação pode continuar fechado.

**Permite:** usar o Cerne em projetos fechados e misturar com código de qualquer licença.

**Exige, ao distribuir:** que os arquivos originalmente MPL continuem MPL, com o código-fonte disponível, inclusive com as modificações.

**Não exige:** abrir os arquivos próprios da aplicação. Um `order.rs` escrito pelo usuário não é um arquivo do Cerne.

**Patentes:** tem concessão de patentes e cláusula de retaliação, parecidas com as da Apache-2.0.

**Quando é acionada:** só na distribuição. Rodar em servidor não aciona.

**Quem usa:** Firefox, Thunderbird.

---

## GPL-3.0 (copyleft forte)

**Em uma frase:** quem distribuir um programa que **inclui** o Cerne precisa distribuir o programa inteiro sob a GPL-3.0, com o código-fonte.

**O detalhe que importa para Rust:** o Rust compila as dependências **dentro** do binário final (link estático). Um binário que usa o Cerne é, para a GPL, uma obra derivada, e o programa todo fica sujeito à GPL ao ser distribuído.

**Quando é acionada:** só na distribuição. Uma API rodando no servidor da própria empresa **não** aciona a GPL; uma ferramenta de linha de comando ou um app desktop vendidos, sim.

**Patentes:** tem concessão de patentes e regras contra "tivoização" (hardware que impede rodar versões modificadas).

**Efeito prático para o Cerne:** quem faz APIs em servidor pouco seria afetado. Mesmo assim, muitas empresas proíbem dependências GPL por política interna, o que reduz a adoção.

**Quem usa:** Linux (GPL-2.0), Git (GPL-2.0), GIMP.

---

## AGPL-3.0 (GPL estendida para a rede)

**Em uma frase:** igual à GPL-3.0, mas a obrigação de publicar o código também vale quando **usuários interagem com o programa pela rede**.

A AGPL foi criada para fechar a brecha do servidor descrita na seção "O que aciona as obrigações". Se uma versão modificada roda num servidor e as pessoas acessam pela rede, quem a opera precisa oferecer o código-fonte a esses usuários.

**Efeito prático para o Cerne:** como o Cerne é feito para construir APIs, a AGPL alcançaria praticamente todo usuário. Na prática, a maioria das empresas proíbe AGPL.

**Uso estratégico:** projetos que querem vender o software usam AGPL para a versão gratuita e vendem uma licença comercial para quem não quer abrir o código ("licença dupla comercial").

**Quem usa:** Mastodon, Grafana, Nextcloud.

---

## Comparação lado a lado

| | MIT | Apache-2.0 | MIT OR Apache | MPL-2.0 | GPL-3.0 | AGPL-3.0 |
|---|---|---|---|---|---|---|
| Usar numa API fechada em servidor | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ |
| Distribuir um binário fechado | ✅ | ✅ | ✅ | ✅ | ❌ | ❌ |
| Publicar modificações feitas no Cerne | Não | Não | Não | Sim, ao distribuir | Sim, ao distribuir | Sim, ao distribuir ou servir pela rede |
| Publicar o código da aplicação inteira | Não | Não | Não | Não | Sim, ao distribuir | Sim, ao distribuir ou servir pela rede |
| Concessão de patentes | ❌ | ✅ | ✅ (pela Apache) | ✅ | ✅ | ✅ |
| Retaliação contra processo de patente | ❌ | ✅ | ✅ (pela Apache) | ✅ | ✅ | ✅ |
| Fala de marca | ❌ | ✅ | ✅ (pela Apache) | ✅ | ❌ | ❌ |
| Compatível com GPL-2.0 | ✅ | ❌ | ✅ (pela MIT) | ✅ | ❌ | ❌ |
| Aceitação em empresas | Muito alta | Muito alta | Muito alta | Alta | Baixa | Muito baixa |

---

## Cenários concretos com o Cerne

| Situação | MIT OR Apache-2.0 | MPL-2.0 | GPL-3.0 | AGPL-3.0 |
|---|---|---|---|---|
| Uma fintech cria a API dela com o Cerne e não publica nada | Pode | Pode | Pode | **Precisa publicar** |
| Uma empresa vende um CLI fechado feito com o Cerne | Pode | Pode | **Precisa publicar** | **Precisa publicar** |
| Alguém corrige um bug no Cerne num fork privado e usa no próprio servidor | Pode guardar a correção | Pode guardar | Pode guardar | **Precisa publicar** |
| Alguém corrige um bug no Cerne e distribui o fork | Pode guardar a correção | **Precisa publicar a correção** | **Precisa publicar** | **Precisa publicar** |
| Um contribuidor envia um PR e depois processa usuários por patente | Perde a licença de patentes (se a outra parte usar Apache) | Perde | Perde | Perde |

---

## Recomendação

**MIT OR Apache-2.0.**

1. **O objetivo é adoção:** o Cerne é um framework, e quanto menos ele exigir de quem o usa, mais gente vai usá-lo.
2. **Proteção contra patentes:** a parte Apache-2.0 protege os usuários e desestimula processos contra o projeto.
3. **É o que o ecossistema espera:** quem vem do Rust reconhece a combinação na hora, e nenhum jurídico vai barrá-la.
4. **Marca protegida:** a Apache-2.0 deixa claro que o nome "Cerne" não está licenciado.

**Quando escolher outra:**
- **MPL-2.0:** se você quiser **obrigar** quem distribui um fork a devolver as melhorias. O custo é um pouco de atrito com empresas.
- **AGPL-3.0 + licença comercial:** se houver um plano de **negócio** para vender o Cerne. É uma decisão de modelo de negócio, não técnica.

---

## Um cuidado específico do Cerne: o código gerado

Os templates do `cerne new` e do `cerne g` fazem parte do Cerne, então estão sob a licença dele. Sem um aviso explícito, alguém poderia argumentar que o código gerado também está, e que precisa manter os avisos de copyright do Cerne.

Para evitar a dúvida, o README deve declarar:

> O código gerado pelo `cerne new` e pelo `cerne g` pertence a quem o gerou, que pode usá-lo sob qualquer licença, sem obrigações com o Cerne.

---

## Checklist depois de decidir (MIT OR Apache-2.0)

1. Adicionar `LICENSE-MIT` e `LICENSE-APACHE` na raiz do repositório, com o texto oficial de cada licença.
2. Em cada `Cargo.toml`: `license = "MIT OR Apache-2.0"`.
3. No README:
   - uma seção "Licença" citando as duas;
   - a frase padrão sobre contribuições;
   - a frase sobre o código gerado pertencer ao usuário.
4. Opcional: um arquivo `NOTICE` com os créditos que você quer que apareçam em produtos derivados.

---

## Decisão tomada: Apache-2.0

O Cerne usa **somente Apache-2.0** (não a licença dupla). O que muda em relação ao checklist acima:

1. ✅ **Um arquivo só:** `LICENSE`, com o texto oficial baixado de apache.org.
2. ✅ **`Cargo.toml`:** `license = "Apache-2.0"`.
3. **README:**
   - uma seção "Licença";
   - a frase sobre o código gerado pertencer a quem o gerou.

   A frase padrão sobre contribuições não é necessária: a seção 5 da Apache-2.0 já cobre isso.
4. **Opcional:** um arquivo `NOTICE`.
5. **Único efeito colateral:** projetos GPL-2.0 não poderão usar o Cerne.
