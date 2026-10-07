# As chamadas da MetaMask

Referência do que a MetaMask manda para o nó de uma rede custom, e do que ela espera receber de volta. A rde é chamada pela MetaMask (**D38**), então este é o contrato que o endpoint JSON-RPC da rde cumpre: o `eth_sendRawTransaction` vira o `CreateTransferCommand`, e os métodos de leitura estão em `examples/rde/src/infrastructure/http/eth.rs`.

O que está aqui foi **capturado**, não tirado da documentação. Onde o texto vai além do que a captura mostrou, ele diz que é inferência. Um servidor de teste em `localhost:8546` gravou cada requisição crua e respondeu com valores plausíveis, enquanto a MetaMask adicionava a rede, lia o saldo e enviava uma transferência.

| | |
|---|---|
| Data da captura | 2026-10-07 |
| Carteira | MetaMask (extensão `nkbihfbeogaeaoehlefnkodbefgpgknn`), instalação nova, no Brave 154 (Linux) |
| Rede | Chain ID `8808` (`0x2268`), moeda `RDEC`, RPC `http://localhost:8546` |
| Volume | 60 requisições, 14 métodos, cerca de 7 minutos |
| Segunda captura | 2026-10-07, mesma carteira: confirmação, "Cancelar" e "Acelerar" (seções 5 a 7), contra um servidor que minera blocos sob comando |

Os endereços dos exemplos foram trocados por `0x1111…` (a conta da MetaMask) e `0x2222…` (o destinatário). As assinaturas foram omitidas.

---

## O envelope

Isto vale para todas as chamadas capturadas.

| | O que a MetaMask manda |
|---|---|
| Verbo HTTP | Sempre `POST`, no caminho `/`. Nenhum `GET`, nenhum preflight `OPTIONS`. |
| `Content-Type` | `application/json`. |
| `Accept` | `application/json`. Só as duas primeiras chamadas (`eth_chainId`, ao adicionar a rede) usaram `*/*`. |
| `Origin` | `chrome-extension://nkbihfbeogaeaoehlefnkodbefgpgknn`. A extensão tem permissão de host, então o navegador não pede CORS. Um dApp que chama o nó direto de uma página pediria. |
| `"jsonrpc"` | Sempre `"2.0"`. |
| `params` | **Sempre array (por posição)**, nunca objeto. Uma posição pode levar um objeto (`eth_call`, `eth_estimateGas`). O `net_version` veio **sem o campo `params`**. |
| `id` | Tipo misturado: string (`"G4H5mcvUUPZu47_nZpsCC"`, `"1791404273057"`) ou número inteiro grande (`7012491944213331`, perto de 2^53). O nó devolve o `id` exatamente como recebeu. |
| Lotes | Nenhum. Cada requisição leva uma chamada só, não um array de chamadas. |

Exemplos reais:

```json
{"id": "1791404273057", "jsonrpc": "2.0", "method": "eth_chainId", "params": []}
{"id": "hPlnKW6QX9vSHNPnnjJXX", "jsonrpc": "2.0", "method": "net_version"}
{"id": 1692846365451518, "jsonrpc": "2.0", "method": "eth_getBlockByNumber", "params": ["0x1", false]}
```

### Números e blocos

- **Quantidades** (saldo, gas, nonce, número do bloco) vão e voltam em hexadecimal com prefixo `0x` e sem zeros à esquerda: `"0x1"`, `"0x5208"`, `"0xde0b6b3a7640000"` (1 RDEC = 10^18 wei).
- **O parâmetro de bloco** é o número do último bloco que o nó informou no `eth_blockNumber` (`"0x1"` na captura), não a palavra `"latest"`. O nó precisa aceitar um número de bloco, não só `"latest"`.

---

## Os fluxos

### 1. Adicionar a rede

Quando o usuário digita a URL da RPC, a MetaMask chama `eth_chainId` para confirmar que o Chain ID bate com o digitado. Sem resposta, a tela mostra "Não foi possível obter o ID da cadeia. O URL da RPC está correto?".

```
eth_chainId            []                     → "0x2268"
eth_chainId            []                     → "0x2268"
eth_blockNumber        []                     → "0x1"
eth_getBalance         ["0x1111…", "0x1"]     → "0xde0b6b3a7640000"
eth_getBlockByNumber   ["0x1", false]         → { bloco, ver abaixo }
net_version            (sem params)           → "8808"
```

Repare que o `net_version` devolve o Chain ID **em decimal e como string**, e o `eth_chainId` devolve em hexadecimal.

### 2. Em repouso

Com a rede selecionada, a MetaMask chama `eth_blockNumber` **a cada 20 segundos**. Quando o número do bloco muda, ela relê o que depende dele (saldo, recibos pendentes).

### 3. Preparar um envio

Ao digitar o destinatário e o valor, a MetaMask verifica se o destinatário é um contrato, e qual:

```
eth_call      [{"to": "0x2222…", "data": "0x01ffc9a780ac58cd…"}, "0x1"]   supportsInterface(ERC-721)
eth_call      [{"to": "0x2222…", "data": "0x01ffc9a7d9b67a26…"}, "0x1"]   supportsInterface(ERC-1155)
eth_call      [{"to": "0x2222…", "data": "0x95d89b41"}, "0x1"]           symbol()
eth_call      [{"to": "0x2222…", "data": "0x313ce567"}, "0x1"]           decimals()
eth_call      [{"to": "0x2222…", "data": "0x70a08231…1111"}, "0x1"]      balanceOf(conta)
eth_call      [{"data": "0x95d89b41"}, "0x1"]                             symbol(), sem "to"
eth_getCode   ["0x1111…", "0x1"]                                          a conta é contrato?
eth_getCode   ["0x2222…", "0x1"]                                          o destinatário é contrato?
eth_gasPrice  []
eth_estimateGas [{"from": "0x1111…", "to": "0x2222…", "value": "0x16345785d8a0000", "data": "0x", "type": "0x2"}]
```

Para um endereço comum (sem contrato), a resposta certa é `"0x"` no `eth_call` e no `eth_getCode`. Com essas respostas, a MetaMask trata o envio como uma transferência da moeda nativa.

O `eth_estimateGas` pede `"type": "0x2"`, então a transação vai ser EIP-1559. Se o saldo não cobre o valor mais a taxa, a MetaMask para aqui e mostra "Insufficient funds", sem enviar nada.

### 4. Enviar

```
eth_getTransactionCount  ["0x1111…", "0x1"]   → "0x0"          o nonce
eth_sendRawTransaction   ["0x02f874…"]        → "0x<hash>"     a transação assinada
```

A MetaMask **assina a transação localmente** e manda só o resultado, em RLP hexadecimal. Nenhum campo vem em JSON: o nó decodifica o RLP para saber quem envia, para quem e quanto. Decodificada, a transação capturada (EIP-1559, o primeiro byte `0x02`) traz:

| Campo | Valor capturado |
|---|---|
| `chainId` | `0x2268` (8808) |
| `nonce` | `0` |
| `maxPriorityFeePerGas` | `0x3b9aca00` (1 gwei) |
| `maxFeePerGas` | `0x3b9aca00` (1 gwei) |
| `gasLimit` | `0x5208` (21000) |
| `to` | `0x2222…` |
| `value` | `0x16345785d8a0000` (0,1 RDEC) |
| `data` | vazio |
| `accessList` | vazia |
| `yParity`, `r`, `s` | a assinatura; o remetente sai dela, não vem escrito |

O `result` do `eth_sendRawTransaction` é o hash da transação (`0x` + 64 dígitos hexadecimais). A MetaMask usa esse hash em todas as chamadas seguintes.

### 5. Acompanhar a transação

Logo depois do envio, a MetaMask chama este par **a cada 3 segundos**, por cerca de 30 segundos:

```
eth_getTransactionReceipt  ["0x<hash>"]   → null enquanto pendente; o recibo quando está num bloco
eth_getTransactionByHash   ["0x<hash>"]
```

Depois, ela segue só o `eth_blockNumber`, a cada 20 segundos, e pede o recibo de novo **cada vez que o número do bloco muda**. Um nó que deixa uma transação pendente por mais de 30 segundos só a vê confirmada no próximo bloco.

A MetaMask dá a transação por confirmada quando o recibo traz `status: "0x1"`, `blockNumber` e `blockHash`. Logo em seguida ela pede o bloco pelo hash (`eth_getBlockByHash`, para o `baseFeePerGas` e o `timestamp`) e o saldo da conta. O recibo que ela aceitou (os endereços abreviados):

```json
{
  "transactionHash": "0x35b8…4634", "transactionIndex": "0x0",
  "blockHash": "0x3437…7320", "blockNumber": "0x4",
  "from": "0x1111…", "to": "0x2222…",
  "status": "0x1", "gasUsed": "0x5208", "cumulativeGasUsed": "0x5208",
  "effectiveGasPrice": "0x3b9aca00", "type": "0x2",
  "contractAddress": null, "logs": [], "logsBloom": "0x00…00"
}
```

A MetaMask acompanha a transação pelo hash que **o nó devolveu** no `eth_sendRawTransaction`, não pelo que ela mesma calcula. Na primeira captura, o servidor de teste devolvia sempre o mesmo hash falso. Duas transações com o mesmo hash travaram o rastreador da MetaMask: ela parou de pedir recibos, inclusive de transações novas, até a extensão ser recarregada (desligar e religar em `brave://extensions`). O nó precisa devolver o hash de verdade, o keccak dos bytes assinados.

### 6. Cancelar

Ao cancelar uma transação pendente, a MetaMask manda outra pelo `eth_sendRawTransaction`, sem chamar antes o `eth_estimateGas` nem o `eth_getTransactionCount`. A capturada, decodificada:

| Campo | Valor capturado |
|---|---|
| `nonce` | o **mesmo** da transação cancelada |
| `maxFeePerGas`, `maxPriorityFeePerGas` | `0x4190ab00` (1,1 gwei, 10% acima) |
| `to` | a própria conta (`0x1111…`); o remetente recuperado da assinatura é essa mesma conta |
| `value` | `0` |
| `gasLimit` | `21000` |

A partir daí, a MetaMask acompanha **as duas**: pede o recibo da transferência e o do cancelamento. Para o cancelamento valer, o nó põe o cancelamento num bloco e devolve o recibo dele com `status: "0x1"`. A transferência fica sem recibo (`null`), e o `eth_getTransactionCount` passa do nonce dela. Com isso, a MetaMask mostra a transferência como **"Falhou", sem motivo**. Ela não mostra "Cancelada".

Na tela, enquanto o cancelamento está pendente, a transferência aparece com um botão **"Acelerar esse cancelamento"**. Clicado, ele não assinou uma transação nova: a MetaMask mandou de novo **os mesmos bytes** do cancelamento, com o mesmo hash. O nó precisa aceitar uma transação que já tem e devolver o mesmo hash, como um nó Ethereum faz.

Uma transferência nova, enviada enquanto outra está pendente, aparece como **"Na fila"**.

### 7. Acelerar

O "Acelerar" de uma transferência pendente manda outra pelo `eth_sendRawTransaction`:

| Campo | Valor capturado |
|---|---|
| `nonce` | o **mesmo** da transferência |
| `maxFeePerGas`, `maxPriorityFeePerGas` | 1,1 gwei, 10% acima |
| `to`, `value` | os **mesmos** da transferência |

Quando a nova entra num bloco, a MetaMask a dá por confirmada, e a original fica sem recibo. Na rde, o "Acelerar" de uma transferência é recusado (**D40**): a taxa não muda a ordem de nada, e a regra "sender has no open transfer" já cobre esse caso.

---

## Os métodos

O que o nó precisa responder, em ordem de quando aparece. A coluna "Sem ele" diz o que aconteceu quando o servidor de teste não respondeu o método, ou respondeu `-32601` (método não encontrado).

| Método | `params` | `result` esperado | Sem ele |
|---|---|---|---|
| `eth_chainId` | `[]` | Chain ID em hexadecimal: `"0x2268"` | A rede não pode ser adicionada |
| `net_version` | ausente | Chain ID em decimal, string: `"8808"` | Não testado |
| `eth_blockNumber` | `[]` | Último bloco: `"0x1"` | Não testado |
| `eth_getBalance` | `[endereço, bloco]` | Saldo em wei: `"0xde0b6b3a7640000"` | Não testado |
| `eth_getBlockByNumber` | `[bloco, false]` | O bloco, só com os hashes das transações (ver abaixo) | Não testado |
| `eth_call` | `[{to?, data}, bloco]` | `"0x"` para quem não é contrato | Não testado |
| `eth_getCode` | `[endereço, bloco]` | `"0x"` para quem não é contrato | Não testado |
| `eth_gasPrice` | `[]` | Preço em wei: `"0x3b9aca00"` | Não testado |
| `eth_estimateGas` | `[{from, to, value, data, type}]` | Gas: `"0x5208"` (21000 numa transferência simples) | Não testado |
| `eth_getTransactionCount` | `[endereço, bloco]` | Próximo nonce da conta: `"0x0"` | Não testado |
| `eth_sendRawTransaction` | `["0x02…"]` | Hash da transação | Não envia |
| `eth_getTransactionReceipt` | `[hash]` | `null` (pendente) ou o recibo (seção 5) | Fica pendente |
| `eth_getTransactionByHash` | `[hash]` | `null` ou a transação | Recebeu `-32601` e seguiu chamando, sem erro na tela |
| `eth_getBlockByHash` | `[hash do bloco, false]` | O bloco, como no `eth_getBlockByNumber` | Não testado; pedido logo depois de um recibo com `status: "0x1"` |

O `eth_feeHistory` e o `eth_maxPriorityFeePerGas` estavam prontos no servidor de teste, mas a MetaMask não os chamou nesta captura. A taxa da transação enviada (1 gwei) é igual ao `eth_gasPrice` e ao `baseFeePerGas` do bloco que o servidor devolveu.

### O bloco do `eth_getBlockByNumber`

O servidor de teste respondeu com este bloco, e a MetaMask aceitou. Não foi testado quais campos ela exige; a captura só mostra que este conjunto basta. Como ela pede transações EIP-1559, o `baseFeePerGas` provavelmente é o que ela lê para a taxa.

```json
{
  "number": "0x1", "hash": "0x11…11", "parentHash": "0x00…00",
  "timestamp": "0x…", "gasLimit": "0x1c9c380", "gasUsed": "0x0",
  "baseFeePerGas": "0x3b9aca00", "miner": "0x00…00", "difficulty": "0x0",
  "transactions": [], "uncles": [], "nonce": "0x0000000000000000",
  "logsBloom": "0x00…00", "extraData": "0x", "size": "0x200",
  "stateRoot": "0x00…00", "receiptsRoot": "0x00…00", "transactionsRoot": "0x00…00",
  "sha3Uncles": "0x00…00", "mixHash": "0x00…00", "totalDifficulty": "0x0"
}
```

---

## O que ainda não foi capturado

- **Um dApp chamando pela MetaMask** (`window.ethereum.request`). O esperado, pela documentação da MetaMask: `eth_requestAccounts`, `personal_sign` e `eth_signTypedData_v4` ficam dentro da carteira, e o `eth_sendTransaction` do dApp chega ao nó como o `eth_sendRawTransaction` deste documento.
- **Tokens (ERC-20):** a MetaMask lê saldo e símbolo pelo `eth_call` (`balanceOf`, `symbol`, `decimals`), mas a captura só viu essas chamadas com resposta `"0x"`.
- **Trocar de rede e voltar**, e a MetaMask no celular.

Para capturar de novo, os servidores de teste são scripts Python que gravam cada requisição e a resposta num arquivo `.jsonl`. O da primeira captura responde aos métodos da tabela acima com valores fixos. O da segunda é uma chain mínima: calcula o hash (keccak), recupera o remetente da assinatura, guarda nonces e saldos, e só minera um bloco quando recebe `POST /mine`, o que dá tempo de clicar em "Cancelar" ou "Acelerar". Num bloco, entre transações de mesmo remetente e mesmo nonce, entra a de taxa maior. Os scripts não estão no repositório.
