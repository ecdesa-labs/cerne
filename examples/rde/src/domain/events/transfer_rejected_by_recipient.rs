use crate::application::commands::chain_failed_transfer::ChainFailedTransferCommand;
use crate::domain::value_objects::tx_hash::TxHash;
use crate::ports::Ports;
use cerne::domain::{DomainEvent, EnforcementResult, FiredPolicy, Policies, Policy};

pub struct TransferRejectedByRecipient {
    pub tx_hash: TxHash,
}

impl DomainEvent<Ports> for TransferRejectedByRecipient {
    fn trigger_policies(&self) -> EnforcementResult<Vec<FiredPolicy<Ports>>> {
        // --- Policies --------------------------------------------------------

        let tx_hash = self.tx_hash.clone();

        let chain_rejected_transfer_policy = Policy::new(
            "whenever a transfer is rejected, chain it as failed",
            || true,
            move || {
                Box::new(ChainFailedTransferCommand {
                    tx_hash: tx_hash.clone(),
                })
            },
        );

        Ok(Policies::new(vec![chain_rejected_transfer_policy]).trigger())
    }
}
