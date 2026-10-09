/// Any error a command, query or port can return: match on the category to decide what to do.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Domain(#[from] DomainError),
    #[error(transparent)]
    Application(#[from] ApplicationError),
    #[error(transparent)]
    Infrastructure(#[from] InfrastructureError),
}

/// The domain refused the change: an invariant or a business rule does not hold.
#[derive(Debug, PartialEq, thiserror::Error)]
pub enum DomainError {
    #[error("violated: {0:?}")]
    Violations(Vec<&'static str>),
}

/// The use case cannot go on, although the domain is fine (e.g. the aggregate does not exist).
#[derive(Debug, PartialEq, thiserror::Error)]
pub enum ApplicationError {
    #[error("{0} not found")]
    NotFound(&'static str),
}

/// Something outside the domain failed: database, network, queue.
#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct InfrastructureError(#[from] anyhow::Error);

/// The outcome of enforcing invariants or checking business rules: the value when they hold, the violated ones otherwise.
pub type EnforcementResult<T> = Result<T, DomainError>;

#[cfg(test)]
mod tests {
    use super::*;

    fn place_order() -> Result<(), Error> {
        Err(DomainError::Violations(vec!["positive quantity"]))?
    }

    #[test]
    fn question_mark_turns_a_domain_error_into_an_error() {
        assert!(matches!(
            place_order(),
            Err(Error::Domain(DomainError::Violations(v))) if v == vec!["positive quantity"]
        ));
    }

    #[test]
    fn infrastructure_error_keeps_the_message_of_its_cause() {
        let error = Error::from(InfrastructureError::from(anyhow::anyhow!("database is down")));

        assert_eq!(error.to_string(), "database is down");
    }
}
