use crate::domain::value_objects::address::Address;
use cerne::{Error, async_trait};

/// Answers whether a wallet has passed KYC.
///
/// Hotspot 3 ("De onde vem o KYC?"): the board does not say which system this is, so it is a port.
#[async_trait]
pub trait KycRegistry: Send + Sync {
    async fn is_verified(&self, wallet: &Address) -> Result<bool, Error>;
}
