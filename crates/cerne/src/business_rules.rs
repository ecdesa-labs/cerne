use crate::errors::{DomainError, EnforcementResult};

/// What must hold for a command to run: a name, as written on the board, and whether it holds. Written with [`business_rule!`](crate::domain::business_rule).
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

/// Runs the business rules of a command: `BusinessRules::check([business_rule!(..), ..])?`.
pub struct BusinessRules;

impl BusinessRules {
    /// Runs every rule and, if any fails, returns `DomainError::Violations` with every failing name, not just
    /// the first.
    pub fn check(rules: impl IntoIterator<Item = BusinessRule>) -> EnforcementResult<()> {
        let violations: Vec<_> = rules
            .into_iter()
            .filter(|rule| !(rule.holds)())
            .map(|rule| rule.name)
            .collect();

        if violations.is_empty() { Ok(()) } else { Err(DomainError::Violations(violations)) }
    }
}

/// A [`BusinessRule`](crate::domain::BusinessRule): `business_rule!("enough stock", enough_stock)`.
///
/// The condition goes in a variable before, named like the sentence on the board; the macro reads it once, right
/// away, and writes `BusinessRule::new(name, move || condition)`.
///
/// ```
/// use cerne::domain::{BusinessRules, business_rule};
///
/// let quantity = 0;
/// let quantity_is_positive = quantity > 0;
///
/// assert!(BusinessRules::check([business_rule!("quantity is positive", quantity_is_positive)]).is_err());
/// ```
#[doc(hidden)]
#[macro_export]
macro_rules! business_rule {
    ($name:expr, $holds:expr $(,)?) => {{
        let holds: bool = $holds;

        $crate::domain::BusinessRule::new($name, move || holds)
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_reports_only_the_violated_rules() {
        let quantity = 2000;
        let quantity_is_positive = quantity > 0;
        let quantity_is_less_than_1000 = quantity < 1000;

        assert_eq!(
            BusinessRules::check([
                business_rule!("positive quantity", quantity_is_positive),
                business_rule!("quantity less than 1000", quantity_is_less_than_1000),
            ]),
            Err(DomainError::Violations(vec!["quantity less than 1000"]))
        );
    }

    #[test]
    fn check_takes_the_rules_a_command_builds_in_a_vec() {
        let enough_stock = false;
        let business_rules = vec![business_rule!("enough stock", enough_stock)];

        assert_eq!(BusinessRules::check(business_rules), Err(DomainError::Violations(vec!["enough stock"])));
    }
}
