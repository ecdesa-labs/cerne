/// What a transfer costs on top of its amount, in RDEC.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fees {
    pub decarbonization: u64,
    pub gas: u64,
}

impl Fees {
    pub fn total(&self) -> u64 {
        self.decarbonization + self.gas
    }
}

/// Domain service "Estimar taxa de descarb e GasFee": a calculation, nothing to violate.
///
/// Placeholder rates (1% decarbonization, 1 RDEC of gas): the real ones are not on the board yet.
pub fn estimate_fees(amount: u64) -> Fees {
    Fees {
        decarbonization: amount / 100,
        gas: 1,
    }
}
