use cerne::application::Repository;
use cerne::domain::Aggregate;
use cerne::{ApplicationError, Error, async_trait};
use std::sync::Mutex;

/// Keeps the aggregates in a `Vec`: good for tests and prototypes, gone when the process ends.
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

        let found = aggregates.iter().find(|a| a.id() == id).cloned();

        Ok(found.ok_or(ApplicationError::NotFound(std::any::type_name::<A>()))?)
    }

    async fn save(&self, aggregate: A) -> Result<(), Error> {
        let mut aggregates = self.0.lock().unwrap();

        aggregates.retain(|a| a.id() != aggregate.id());
        aggregates.push(aggregate);

        Ok(())
    }
}
