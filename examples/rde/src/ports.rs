use crate::application::ports::blockchain::Blockchain;
use crate::application::ports::kyc_registry::KycRegistry;
use crate::application::ports::notifier::Notifier;
use crate::domain::entities::transfer::Transfer;
use cerne::application::Repository;

/// The composition root: every port the transfer commands can use.
pub struct Ports {
    pub transfers: Box<dyn Repository<Transfer>>,
    pub blockchain: Box<dyn Blockchain>,
    pub kyc: Box<dyn KycRegistry>,
    pub notifier: Box<dyn Notifier>,
}
