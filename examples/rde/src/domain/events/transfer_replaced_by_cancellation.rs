use crate::application::commands::chain_cancellation::ChainCancellationCommand;
use crate::domain::value_objects::tx_hash::TxHash;
use crate::ports::Ports;
use cerne::domain::{DomainEvent, EnforcementResult, FiredPolicy, Policies, Policy};

/// The sender canceled a pending transfer in MetaMask: the cancellation replaces it.
pub struct TransferReplacedByCancellation {
    pub tx_hash: TxHash,
    pub cancellation_tx_hash: TxHash,
}

impl DomainEvent<Ports> for TransferReplacedByCancellation {
    fn trigger_policies(&self) -> EnforcementResult<Vec<FiredPolicy<Ports>>> {
        // --- Policies --------------------------------------------------------

        let tx_hash = self.tx_hash.clone();

        let chain_cancellation_policy = Policy::new(
            "whenever a transfer is replaced by a cancellation, chain the cancellation",
            || true,
            move || {
                Box::new(ChainCancellationCommand {
                    tx_hash: tx_hash.clone(),
                })
            },
        );

        Ok(Policies::new(vec![chain_cancellation_policy]).trigger())
    }
}
