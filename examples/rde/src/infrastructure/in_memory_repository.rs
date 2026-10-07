use cerne::application::Repository;
use cerne::domain::Aggregate;
use cerne::{ApplicationError, Error, InfrastructureError, async_trait};
use std::sync::Mutex;

/// Keeps the aggregates in a `Vec`: good for tests and prototypes, gone when the process ends.
///
/// It only stores aggregates that already have an id, like the `Transfer` (its `tx_hash`): deciding ids is the job of
/// the in-memory adapter that comes with `cerne` in Phase 3.
pub struct InMemoryRepository<A>(Mutex<Vec<A>>);

impl<A> InMemoryRepository<A> {
    pub fn new(aggregates: Vec<A>) -> Self {
        Self(Mutex::new(aggregates))
    }
}

#[async_trait]
impl<A: Aggregate + Clone> Repository<A> for InMemoryRepository<A> {
    async fn load(&self, id: &A::Id) -> Result<A, Error> {
        let aggregates = self.0.lock().unwrap();

        let found = aggregates.iter().find(|a| a.id() == Some(id)).cloned();

        Ok(found.ok_or(ApplicationError::NotFound(std::any::type_name::<A>()))?)
    }

    async fn save(&self, aggregate: A) -> Result<A::Id, Error> {
        let mut aggregates = self.0.lock().unwrap();

        let id = aggregate.id().cloned().ok_or_else(|| {
            InfrastructureError::from(anyhow::anyhow!("this repository does not decide ids"))
        })?;

        aggregates.retain(|a| a.id() != Some(&id));
        aggregates.push(aggregate);

        Ok(id)
    }
}
