use crate::domain::value_objects::signed_transaction::SignedTransaction;

/// What a transfer costs on top of its amount, in wei.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fees {
    pub decarbonization: u128,
    pub gas: u128,
}

impl Fees {
    pub fn total(&self) -> u128 {
        self.decarbonization + self.gas
    }
}

/// Domain service "Estimar taxa de descarb e GasFee": a calculation, nothing to violate.
///
/// The gas is what the signed transaction allows (gas limit × max fee per gas), the same MetaMask showed the sender.
/// The decarbonization (1%, a placeholder rate: the real one is not on the board yet) is charged on top, and MetaMask
/// does not show it.
pub fn estimate_fees(signed_transaction: &SignedTransaction) -> Fees {
    Fees {
        decarbonization: signed_transaction.amount() / 100,
        gas: signed_transaction.gas_limit() as u128 * signed_transaction.max_fee_per_gas(),
    }
}
