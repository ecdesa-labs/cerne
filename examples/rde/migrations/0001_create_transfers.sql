-- Amounts and fees are in wei, which passes the 64 bits of a BIGINT: they are stored as decimal text.
CREATE TABLE transfers (
    tx_hash TEXT PRIMARY KEY,
    signed_transaction TEXT NOT NULL,
    sender TEXT NOT NULL,
    recipient TEXT NOT NULL,
    amount TEXT NOT NULL,
    decarbonization_fee TEXT NOT NULL,
    gas_fee TEXT NOT NULL,
    nonce BIGINT NOT NULL,
    status TEXT NOT NULL,
    chained BOOLEAN NOT NULL
);
