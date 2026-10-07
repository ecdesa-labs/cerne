use crate::application::commands::chain_accepted_transfer::ChainAcceptedTransferCommand;
use crate::ports::Ports;
use cerne::domain::{DomainEvent, EnforcementResult, FiredPolicy, Policies, Policy};

pub struct TransferAcceptedByRecipient {
    pub tx_hash: String,
}

impl DomainEvent<Ports> for TransferAcceptedByRecipient {
    fn trigger_policies(&self) -> EnforcementResult<Vec<FiredPolicy<Ports>>> {
        // --- Policies --------------------------------------------------------

        let tx_hash = self.tx_hash.clone();

        let chain_accepted_transfer_policy = Policy::new(
            "whenever a transfer is accepted, chain it",
            || true,
            move || {
                Box::new(ChainAcceptedTransferCommand {
                    tx_hash: tx_hash.clone(),
                })
            },
        );

        Ok(Policies::new(vec![chain_accepted_transfer_policy]).trigger())
    }
}
