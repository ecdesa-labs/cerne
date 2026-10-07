use crate::errors::EnforcementResult;

/// A value with no identity: two value objects with the same fields are the same value.
///
/// It is born valid and never changes. There is no `validate`, because there is no state transition to check:
/// a different value is a new value object, built with `new`.
///
/// ```
/// use cerne::domain::{EnforcementResult, Invariant, Invariants, ValueObject};
///
/// #[derive(Debug, Clone, PartialEq)]
/// struct Amount(u64);
///
/// impl ValueObject for Amount {
///     type Props = u64;
///
///     fn new(value: u64) -> EnforcementResult<Self> {
///         let amount_is_positive = value > 0;
///
///         Invariants::new(vec![Invariant::new("amount is positive", move || {
///             amount_is_positive
///         })])
///         .enforce()?;
///
///         Ok(Self(value))
///     }
/// }
///
/// assert_eq!(Amount::new(100), Amount::new(100));
/// assert!(Amount::new(0).is_err());
/// ```
pub trait ValueObject: Sized + Clone + PartialEq + Send + Sync {
    type Props;

    /// A value object is born valid: it only exists if its invariants hold.
    ///
    /// Otherwise nothing is created, and the error names every violated invariant, not just the first.
    fn new(props: Self::Props) -> EnforcementResult<Self>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::DomainError;
    use crate::invariants::{Invariant, Invariants};

    #[derive(Debug, Clone, PartialEq)]
    struct Email(String);

    impl ValueObject for Email {
        type Props = String;

        fn new(address: String) -> EnforcementResult<Self> {
            let has_an_at_sign = address.contains('@');
            let is_short_enough = address.len() <= 254;

            Invariants::new(vec![
                Invariant::new("email has an @", move || has_an_at_sign),
                Invariant::new("email has at most 254 characters", move || is_short_enough),
            ])
            .enforce()?;

            Ok(Self(address))
        }
    }

    #[test]
    fn value_object_is_born_when_invariants_hold() {
        assert_eq!(
            Email::new("alice@rde.io".into()),
            Ok(Email("alice@rde.io".into()))
        );
    }

    #[test]
    fn value_object_cannot_be_born_invalid() {
        assert_eq!(
            Email::new("alice".into()),
            Err(DomainError::Violations(vec!["email has an @"]))
        );
    }

    #[test]
    fn value_objects_with_the_same_fields_are_equal() {
        assert_eq!(
            Email::new("alice@rde.io".into()),
            Email::new("alice@rde.io".into())
        );
    }
}
