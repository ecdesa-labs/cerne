use crate::domain::services::fees::Fees;
use sha3::{Digest, Keccak256};

/// The id of a transfer: the Keccak-256 of its transaction, as on Polygon (see `HOW_TX_HASH_IS_GENERATED.md`).
///
/// Nobody picks it: the same transaction always has the same hash, before it is in any block.
/// Simplified: hashes the fields joined by `|`, where the chain hashes the RLP of the signed transaction.
pub fn transaction_hash(
    sender: &str,
    recipient: &str,
    amount: u64,
    fees: Fees,
    nonce: u64,
) -> String {
    let transaction = format!(
        "{sender}|{recipient}|{amount}|{}|{}|{nonce}",
        fees.decarbonization, fees.gas
    );

    let hash = Keccak256::digest(transaction.as_bytes());

    let hex: String = hash.iter().map(|byte| format!("{byte:02x}")).collect();
    format!("0x{hex}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::services::fees::estimate_fees;

    #[test]
    fn same_transaction_has_the_same_hash() {
        let fees = estimate_fees(100);

        assert_eq!(
            transaction_hash("alice", "bob", 100, fees, 0),
            transaction_hash("alice", "bob", 100, fees, 0)
        );
    }

    #[test]
    fn any_field_changes_the_hash() {
        let fees = estimate_fees(100);
        let original = transaction_hash("alice", "bob", 100, fees, 0);

        assert_ne!(original, transaction_hash("alice", "bob", 100, fees, 1));
        assert_ne!(original, transaction_hash("alice", "carol", 100, fees, 0));
    }
}
