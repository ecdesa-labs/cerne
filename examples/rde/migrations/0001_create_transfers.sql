CREATE TABLE transfers (
    tx_hash TEXT PRIMARY KEY,
    sender TEXT NOT NULL,
    recipient TEXT NOT NULL,
    amount BIGINT NOT NULL,
    decarbonization_fee BIGINT NOT NULL,
    gas_fee BIGINT NOT NULL,
    nonce BIGINT NOT NULL,
    status TEXT NOT NULL,
    chained BOOLEAN NOT NULL
);
