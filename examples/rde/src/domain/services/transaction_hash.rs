use sha3::{Digest, Keccak256};

/// The id of a transfer: the Keccak-256 of its signed transaction, as on Polygon (see `HOW_TX_HASH_IS_GENERATED.md`).
///
/// Nobody picks it: the same transaction always has the same hash, before it is in any block. `bytes` are the
/// EIP-2718 bytes MetaMask signed (`0x02` + the RLP), so the chain and MetaMask compute the same hash.
pub fn transaction_hash(bytes: &[u8]) -> String {
    let hash = Keccak256::digest(bytes);

    let hex: String = hash.iter().map(|byte| format!("{byte:02x}")).collect();
    format!("0x{hex}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_bytes_have_the_same_hash() {
        assert_eq!(transaction_hash(b"transfer"), transaction_hash(b"transfer"));
    }

    #[test]
    fn any_byte_changes_the_hash() {
        assert_ne!(transaction_hash(b"transfer"), transaction_hash(b"transfeR"));
    }
}
