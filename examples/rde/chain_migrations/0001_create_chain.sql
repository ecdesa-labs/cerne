-- The RDE chain the server stands in for (`SqliteBlockchain`). It lives in a database of its own (`rde-chain.db`):
-- an external system, outside the transaction of the transfers. Amounts are in wei, stored as decimal text.
CREATE TABLE chain_balances (
    wallet TEXT PRIMARY KEY,
    wei TEXT NOT NULL
);

CREATE TABLE chain_nonces (
    wallet TEXT PRIMARY KEY,
    next_nonce BIGINT NOT NULL
);

-- One transaction per block, like the InMemoryBlockchain; the genesis block has none.
CREATE TABLE chain_blocks (
    number BIGINT PRIMARY KEY,
    hash TEXT NOT NULL,
    parent_hash TEXT NOT NULL,
    timestamp BIGINT NOT NULL,
    base_fee_per_gas TEXT NOT NULL,
    tx_hash TEXT
);

CREATE TABLE chain_receipts (
    tx_hash TEXT PRIMARY KEY,
    block_number BIGINT NOT NULL,
    block_hash TEXT NOT NULL,
    sender TEXT NOT NULL,
    recipient TEXT NOT NULL,
    succeeded BOOLEAN NOT NULL,
    gas_used BIGINT NOT NULL,
    gas_price TEXT NOT NULL
);
