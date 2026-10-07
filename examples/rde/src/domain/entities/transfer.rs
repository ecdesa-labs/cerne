use crate::domain::services::fees::Fees;
use crate::domain::services::transaction_hash::transaction_hash;
use crate::domain::value_objects::tx_hash::TxHash;
use cerne::domain::{Aggregate, EnforcementResult, Entity, Invariant, Invariants, ValueObject};

// --- Status ------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TransferStatus {
    Pending,
    Accepted,
    Rejected,
    Canceled,
}

// --- Aggregate ---------------------------------------------------------------

/// A transfer of RDEC between two wallets: Pending → Accepted, Rejected or Canceled.
/// Once accepted, its transaction is sent to the chain.
#[derive(Debug, Clone, PartialEq)]
pub struct Transfer {
    pub tx_hash: TxHash,
    pub sender: String,
    pub recipient: String,
    pub amount: u64,
    pub fees: Fees,
    pub nonce: u64,
    pub status: TransferStatus,
    pub chained: bool,
}

/// Everything that makes up the transaction. There is no id: it is the hash of these fields.
pub struct TransferProps {
    pub sender: String,
    pub recipient: String,
    pub amount: u64,
    pub fees: Fees,
    pub nonce: u64,
}

impl Aggregate for Transfer {}

// --- Entity: identity and invariants -----------------------------------------

impl Entity for Transfer {
    type Id = TxHash;
    type Props = TransferProps;

    /// Always `Some`: the hash exists before the first save, so the repository has no id to decide.
    fn id(&self) -> Option<&TxHash> {
        Some(&self.tx_hash)
    }

    fn with_id(self, tx_hash: TxHash) -> Self {
        Self { tx_hash, ..self }
    }

    /// A transfer is born pending, identified by the hash of its transaction.
    fn new(props: TransferProps) -> EnforcementResult<Self> {
        let tx_hash = TxHash::new(transaction_hash(
            &props.sender,
            &props.recipient,
            props.amount,
            props.fees,
            props.nonce,
        ))?;

        Self {
            tx_hash,
            sender: props.sender,
            recipient: props.recipient,
            amount: props.amount,
            fees: props.fees,
            nonce: props.nonce,
            status: TransferStatus::Pending,
            chained: false,
        }
        .validate()
    }

    fn validate(self) -> EnforcementResult<Self> {
        let amount_is_positive = self.amount > 0;
        let sender_is_not_recipient = self.sender != self.recipient;
        let is_accepted = self.status == TransferStatus::Accepted;
        let not_chained_yet = !self.chained;

        Invariants::new(vec![
            Invariant::new("amount is positive", move || amount_is_positive),
            Invariant::new("sender is not the recipient", move || {
                sender_is_not_recipient
            }),
            Invariant::new("only an accepted transfer is chained", move || {
                not_chained_yet || is_accepted
            }),
        ])
        .enforce()?;

        Ok(self)
    }
}

// --- State transitions -------------------------------------------------------

impl Transfer {
    pub fn accept(self) -> EnforcementResult<Self> {
        self.change_status(TransferStatus::Accepted)
    }

    pub fn reject(self) -> EnforcementResult<Self> {
        self.change_status(TransferStatus::Rejected)
    }

    pub fn cancel(self) -> EnforcementResult<Self> {
        self.change_status(TransferStatus::Canceled)
    }

    pub fn chain(self) -> EnforcementResult<Self> {
        Self {
            chained: true,
            ..self
        }
        .validate()
    }

    fn change_status(self, status: TransferStatus) -> EnforcementResult<Self> {
        Self { status, ..self }.validate()
    }
}
