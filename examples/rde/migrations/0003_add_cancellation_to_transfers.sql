-- The cancellation MetaMask signed for a pending transfer (same nonce, nothing to the sender itself), and its hash,
-- by which MetaMask follows it.
ALTER TABLE transfers ADD COLUMN cancellation TEXT;
ALTER TABLE transfers ADD COLUMN cancellation_tx_hash TEXT;
