use crate::application::commands::chain_failed_transfer::ChainFailedTransferCommand;
use crate::domain::value_objects::tx_hash::TxHash;
use crate::ports::Ports;
use cerne::domain::{DomainEvent, EnforcementResult, FiredPolicy, Policies, Policy};

pub struct TransferCanceledBySender {
    pub tx_hash: TxHash,
}

impl DomainEvent<Ports> for TransferCanceledBySender {
    fn trigger_policies(&self) -> EnforcementResult<Vec<FiredPolicy<Ports>>> {
        // --- Policies --------------------------------------------------------

        let tx_hash = self.tx_hash.clone();

        let chain_canceled_transfer_policy = Policy::new(
            "whenever a transfer is canceled, chain it as failed",
            || true,
            move || {
                Box::new(ChainFailedTransferCommand {
                    tx_hash: tx_hash.clone(),
                })
            },
        );

        Ok(Policies::new(vec![chain_canceled_transfer_policy]).trigger())
    }
}
