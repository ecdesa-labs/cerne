use crate::application::ports::kyc_registry::KycRegistry;
use cerne::{Error, async_trait};
use std::collections::HashSet;

/// The wallets that passed KYC, fixed when the registry is built.
pub struct InMemoryKycRegistry(HashSet<String>);

impl InMemoryKycRegistry {
    pub fn new(verified: Vec<&str>) -> Self {
        Self(verified.into_iter().map(String::from).collect())
    }
}

#[async_trait]
impl KycRegistry for InMemoryKycRegistry {
    async fn is_verified(&self, wallet: &str) -> Result<bool, Error> {
        Ok(self.0.contains(wallet))
    }
}
