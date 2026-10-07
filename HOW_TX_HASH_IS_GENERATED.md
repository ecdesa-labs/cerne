Quando a gente manda uma transação para a polygon, ela nos retorna uma tx_id antes mesmo da tx ser incluida num bloco. Como a tx_id é gerada?

O tx_id (na verdade chamado de transaction hash) não é "gerado" pela rede: ele é calculado deterministicamente a partir do conteúdo da própria transação assinada. Por isso ele existe antes de qualquer bloco.

Como é calculado
Na Polygon PoS (que é compatível com a EVM, igual ao Ethereum), o hash é:

tx_hash = keccak256( serialização_RLP_da_transação_assinada )
Passo a passo:

Você monta a transação com campos como nonce, to, value, data, gasLimit, chainId e as taxas (maxFeePerGas e maxPriorityFeePerGas, no formato EIP-1559).
Você assina (ou sua carteira/biblioteca assina) com sua chave privada. Isso produz os valores v, r, s da assinatura ECDSA.
A transação assinada é serializada em RLP (Recursive Length Prefix). Para transações tipadas (EIP-2718/EIP-1559), o formato é tipo || RLP([...campos, v, r, s]).
Aplica-se o Keccak-256 sobre esses bytes. O resultado de 32 bytes é o hash.
Por que você recebe o hash antes do bloco
Quando você faz eth_sendRawTransaction, o nó:

valida a assinatura, nonce, saldo e gas;
coloca a tx na mempool;
recalcula o keccak256 dos bytes recebidos e devolve esse hash.
Ou seja, o nó só está te devolvendo o identificador que já era determinado pelos bytes que você enviou. Nenhuma informação do bloco (número, posição, validador) entra no cálculo. Você mesmo poderia calcular o hash localmente antes de enviar (por exemplo, com ethers.keccak256(signedTx) ou ethers.Transaction.from(signedTx).hash).

Consequências práticas
Mudou qualquer campo, muda o hash. Isso inclui gasPrice/taxas, nonce ou data. Por isso, quando você faz um speed up (reenvia com taxa maior e mesmo nonce), o resultado é um hash novo, e o antigo fica órfão/substituído.
Hash retornado ≠ tx confirmada. Ele só diz que o nó aceitou a tx na mempool. Ela ainda pode ser descartada, substituída ou ficar pendente. Para confirmar, consulte eth_getTransactionReceipt até aparecer o receipt (e espere algumas confirmações, dado o risco de reorg na Polygon).
A assinatura faz parte do hash. Como o ECDSA usa um valor aleatório/determinístico (RFC 6979) na assinatura, o mesmo conteúdo assinado pela mesma chave tende a gerar sempre o mesmo r,s em bibliotecas modernas, mas o hash depende dos bytes assinados efetivamente enviados.
Em resumo: o tx hash é só o keccak256 da transação assinada, então qualquer um que tenha esses bytes consegue calcular o mesmo ID, sem precisar esperar bloco nenhum.

