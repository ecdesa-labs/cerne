use cerne::domain::{DomainError, EnforcementResult, Invariant, Invariants, ValueObject};
use serde::{Deserialize, Serialize};
use std::fmt;

/// A wallet on the RDE chain: `0x` + 40 hex digits, as MetaMask shows it.
///
/// Kept in lowercase, so the checksummed form (`0x6d13Dbb0…`) and the lowercase one are the same wallet.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Address(String);

impl ValueObject for Address {
    type Props = String;

    fn new(address: String) -> EnforcementResult<Self> {
        let address = address.to_lowercase();
        let digits = address.strip_prefix("0x").unwrap_or_default();

        let starts_with_0x = address.starts_with("0x");
        let has_40_digits = digits.len() == 40;
        let digits_are_hex = digits.chars().all(|digit| digit.is_ascii_hexdigit());

        Invariants::new(vec![
            Invariant::new("address starts with 0x", move || starts_with_0x),
            Invariant::new("address has 40 digits", move || has_40_digits),
            Invariant::new("address digits are hex", move || digits_are_hex),
        ])
        .enforce()?;

        Ok(Self(address))
    }
}

impl TryFrom<String> for Address {
    type Error = DomainError;

    fn try_from(address: String) -> EnforcementResult<Self> {
        Self::new(address)
    }
}

impl From<Address> for String {
    fn from(address: Address) -> String {
        address.0
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checksummed_and_lowercase_are_the_same_wallet() {
        let checksummed = Address::new("0x7E5F4552091A69125d5DfCb7b8C2659029395Bdf".into());
        let lowercase = Address::new("0x7e5f4552091a69125d5dfcb7b8c2659029395bdf".into());

        assert_eq!(checksummed, lowercase);
    }

    #[test]
    fn address_names_every_violation() {
        assert_eq!(
            Address::new("alice".into()),
            Err(DomainError::Violations(vec![
                "address starts with 0x",
                "address has 40 digits"
            ]))
        );
    }
}
