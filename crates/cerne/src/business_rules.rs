use crate::errors::{DomainError, EnforcementResult};

pub struct BusinessRule {
    name: &'static str,
    holds: Box<dyn Fn() -> bool + Send + Sync>,
}

impl BusinessRule {
    pub fn new(name: &'static str, holds: impl Fn() -> bool + Send + Sync + 'static) -> Self {
        Self {
            name,
            holds: Box::new(holds),
        }
    }
}

pub struct BusinessRules(Vec<BusinessRule>);

impl BusinessRules {
    pub fn new(rules: Vec<BusinessRule>) -> Self {
        Self(rules)
    }

    pub fn check(&self) -> EnforcementResult<()> {
        let violations: Vec<_> = self
            .0
            .iter()
            .filter(|r| !(r.holds)())
            .map(|r| r.name)
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
    fn check_reports_the_violated_rule() {
        let qty = -1;

        let business_rules = BusinessRules::new(vec![
            BusinessRule::new("positive quantity", move || qty > 0),
            BusinessRule::new("always ok", || true),
        ]);

        assert_eq!(
            business_rules.check(),
            Err(DomainError::Violations(vec!["positive quantity"]))
        );
    }

    #[test]
    fn check_reports_only_the_violated_rules() {
        let qty = 2000;

        let business_rules = BusinessRules::new(vec![
            BusinessRule::new("positive quantity", move || qty > 0),
            BusinessRule::new("quantity less than 1000", move || qty < 1000),
            BusinessRule::new("always ok", || true),
        ]);

        assert_eq!(
            business_rules.check(),
            Err(DomainError::Violations(vec!["quantity less than 1000"]))
        );
    }
}
