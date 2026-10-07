use crate::application::ports::kyc_registry::KycRegistry;
use crate::domain::value_objects::address::Address;
use cerne::{Error, async_trait};
use std::collections::HashSet;

/// The wallets that passed KYC, fixed when the registry is built.
pub struct InMemoryKycRegistry(HashSet<Address>);

impl InMemoryKycRegistry {
    pub fn new(verified: Vec<Address>) -> Self {
        Self(verified.into_iter().collect())
    }
}

#[async_trait]
impl KycRegistry for InMemoryKycRegistry {
    async fn is_verified(&self, wallet: &Address) -> Result<bool, Error> {
        Ok(self.0.contains(wallet))
    }
}
