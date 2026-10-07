use cerne::domain::{DomainError, EnforcementResult, Invariant, Invariants, ValueObject};
use serde::{Deserialize, Serialize};
use std::fmt;

/// The id of a transfer: the Keccak-256 of its transaction, written as `0x` + 64 hex digits.
///
/// Two transfers with the same hash are the same transaction, whoever computed it.
/// In JSON it is the string, and reading it back goes through `new`: the invariants hold there too.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct TxHash(String);

impl ValueObject for TxHash {
    type Props = String;

    fn new(hash: String) -> EnforcementResult<Self> {
        let digits = hash.strip_prefix("0x").unwrap_or_default();

        let starts_with_0x = hash.starts_with("0x");
        let has_64_digits = digits.len() == 64;
        let digits_are_hex = digits.chars().all(|digit| digit.is_ascii_hexdigit());

        Invariants::new(vec![
            Invariant::new("tx hash starts with 0x", move || starts_with_0x),
            Invariant::new("tx hash has 64 digits", move || has_64_digits),
            Invariant::new("tx hash digits are hex", move || digits_are_hex),
        ])
        .enforce()?;

        Ok(Self(hash))
    }
}

impl TryFrom<String> for TxHash {
    type Error = DomainError;

    fn try_from(hash: String) -> EnforcementResult<Self> {
        Self::new(hash)
    }
}

impl From<TxHash> for String {
    fn from(tx_hash: TxHash) -> String {
        tx_hash.0
    }
}

impl fmt::Display for TxHash {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cerne::domain::DomainError;

    #[test]
    fn tx_hash_is_0x_and_64_hex_digits() {
        let hash = format!("0x{}", "ab".repeat(32));

        assert_eq!(TxHash::new(hash.clone()).unwrap().to_string(), hash);
    }

    #[test]
    fn tx_hash_names_every_violation() {
        assert_eq!(
            TxHash::new("hash".into()),
            Err(DomainError::Violations(vec![
                "tx hash starts with 0x",
                "tx hash has 64 digits"
            ]))
        );
    }
}
