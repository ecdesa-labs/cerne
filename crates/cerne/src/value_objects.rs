use crate::errors::EnforcementResult;

/// A value with no identity: two value objects with the same fields are the same value.
///
/// It is born valid and never changes. There is no `validate`, because there is no state transition to check:
/// a different value is a new value object, built with `new`.
///
/// On a struct with one unnamed field, [`value_object`](crate::domain::value_object) writes the conversions to and
/// from the value it wraps; `new`, with the invariants, is yours.
///
/// ```
/// use cerne::domain::{EnforcementResult, Invariants, ValueObject, invariant};
///
/// #[derive(Debug, Clone, PartialEq)]
/// struct Amount(u64);
///
/// impl ValueObject for Amount {
///     type Constructor = u64;
///
///     fn new(value: u64) -> EnforcementResult<Self> {
///         let amount_is_positive = value > 0;
///
///         Invariants::enforce([invariant!("amount is positive", amount_is_positive)])?;
///
///         Ok(Self(value))
///     }
/// }
///
/// assert_eq!(Amount::new(100), Amount::new(100));
/// assert!(Amount::new(0).is_err());
/// ```
pub trait ValueObject: Sized + Clone + PartialEq + Send + Sync {
    /// What a new value object is made of: the value it wraps (`u64` in `OrderId(u64)`), or a `<Name>Constructor`
    /// with its fields.
    type Constructor;

    /// A value object is born valid: it only exists if its invariants hold.
    ///
    /// Otherwise nothing is created, and the error names every violated invariant, not just the first.
    fn new(constructor: Self::Constructor) -> EnforcementResult<Self>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::DomainError;
    use crate::invariant;
    use crate::invariants::Invariants;
    use cerne_macros::value_object;
    use serde::{Deserialize, Serialize};

    #[value_object]
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct Email(String);

    impl ValueObject for Email {
        type Constructor = String;

        fn new(address: String) -> EnforcementResult<Self> {
            let has_an_at_sign = address.contains('@');
            let is_short_enough = address.len() <= 254;

            Invariants::enforce([
                invariant!("email has an @", has_an_at_sign),
                invariant!("email has at most 254 characters", is_short_enough),
            ])?;

            Ok(Self(address))
        }
    }

    #[test]
    fn value_object_is_born_when_invariants_hold() {
        assert_eq!(Email::new("alice@example.com".into()), Ok(Email("alice@example.com".into())));
    }

    #[test]
    fn value_object_cannot_be_born_invalid() {
        assert_eq!(Email::new("alice".into()), Err(DomainError::Violations(vec!["email has an @"])));
    }

    #[test]
    fn value_objects_with_the_same_fields_are_equal() {
        assert_eq!(Email::new("alice@example.com".into()), Email::new("alice@example.com".into()));
    }

    #[test]
    fn value_object_converts_to_and_from_the_value_it_wraps() {
        let email = Email::try_from(String::from("alice@example.com")).unwrap();

        assert_eq!(String::from(email), "alice@example.com");
        assert!(Email::try_from(String::from("alice")).is_err());
    }

    #[test]
    fn value_object_is_the_value_itself_in_json_and_is_read_back_through_new() {
        let email = Email::new("alice@example.com".into()).unwrap();

        assert_eq!(serde_json::to_string(&email).unwrap(), r#""alice@example.com""#);
        assert_eq!(serde_json::from_str::<Email>(r#""alice@example.com""#).unwrap(), email);
        assert!(serde_json::from_str::<Email>(r#""alice""#).is_err());
    }
}
