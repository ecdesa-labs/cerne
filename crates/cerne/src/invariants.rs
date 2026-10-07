use crate::errors::{DomainError, EnforcementResult};

pub struct Invariant {
    name: &'static str,
    holds: Box<dyn Fn() -> bool + Send + Sync>,
}

impl Invariant {
    pub fn new(name: &'static str, holds: impl Fn() -> bool + Send + Sync + 'static) -> Self {
        Self {
            name,
            holds: Box::new(holds),
        }
    }
}

pub struct Invariants(Vec<Invariant>);

impl Invariants {
    pub fn new(invariants: Vec<Invariant>) -> Self {
        Self(invariants)
    }

    pub fn enforce(&self) -> EnforcementResult<()> {
        let violations: Vec<_> = self
            .0
            .iter()
            .filter(|i| !(i.holds)())
            .map(|i| i.name)
            .collect();

        if violations.is_empty() {
            Ok(())
        } else {
            Err(DomainError::Violations(violations))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enforce_is_ok_when_all_hold() {
        let invariants = Invariants::new(vec![Invariant::new("always ok", || true)]);

        assert_eq!(invariants.enforce(), Ok(()));
    }

    #[test]
    fn enforce_returns_every_violation() {
        let invariants = Invariants::new(vec![
            Invariant::new("a", || false),
            Invariant::new("b", || false),
        ]);

        assert_eq!(
            invariants.enforce(),
            Err(DomainError::Violations(vec!["a", "b"]))
        );
    }
}
