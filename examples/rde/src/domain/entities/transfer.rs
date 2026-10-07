use crate::domain::services::fees::Fees;
use crate::domain::value_objects::address::Address;
use crate::domain::value_objects::signed_transaction::SignedTransaction;
use crate::domain::value_objects::tx_hash::TxHash;
use cerne::domain::{Aggregate, EnforcementResult, Entity, Invariant, Invariants};

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
/// Once it leaves Pending, its transaction goes to the chain: accepted, it moves the RDEC; rejected or canceled, it
/// goes in as failed, only so the nonce of the sender moves on (MetaMask waits for that). Canceled in MetaMask, the
/// cancellation it signed goes to the chain in its place.
#[derive(Debug, Clone, PartialEq)]
pub struct Transfer {
    pub tx_hash: TxHash,
    pub signed_transaction: SignedTransaction,
    pub sender: Address,
    pub recipient: Address,
    /// In wei, like every amount: 1 RDEC is 10^18 wei.
    pub amount: u128,
    pub fees: Fees,
    pub nonce: u64,
    pub status: TransferStatus,
    pub chained: bool,
    /// The transaction MetaMask signed to cancel this one: same nonce, nothing to the sender itself.
    pub cancellation: Option<SignedTransaction>,
}

/// The transaction the sender signed, and what it costs on top. There is no id: it is the hash of the transaction.
pub struct TransferProps {
    pub signed_transaction: SignedTransaction,
    pub fees: Fees,
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

    /// A transfer is born pending, with what the signed transaction says: who sends, to whom, how much.
    fn new(props: TransferProps) -> EnforcementResult<Self> {
        let signed_transaction = props.signed_transaction;

        Self {
            tx_hash: signed_transaction.tx_hash().clone(),
            sender: signed_transaction.sender().clone(),
            recipient: signed_transaction.recipient().clone(),
            amount: signed_transaction.amount(),
            nonce: signed_transaction.nonce(),
            signed_transaction,
            fees: props.fees,
            status: TransferStatus::Pending,
            chained: false,
            cancellation: None,
        }
        .validate()
    }

    fn validate(self) -> EnforcementResult<Self> {
        let amount_is_positive = self.amount > 0;
        let sender_is_not_recipient = self.sender != self.recipient;
        let is_pending = self.status == TransferStatus::Pending;
        let not_chained_yet = !self.chained;
        let is_canceled = self.status == TransferStatus::Canceled;
        let cancellation_has_the_nonce_of_the_transfer = self
            .cancellation
            .as_ref()
            .is_none_or(|cancellation| cancellation.nonce() == self.nonce);
        let cancellation_comes_from_the_sender = self
            .cancellation
            .as_ref()
            .is_none_or(|cancellation| cancellation.sender() == &self.sender);
        let has_no_cancellation = self.cancellation.is_none();

        Invariants::new(vec![
            Invariant::new("amount is positive", move || amount_is_positive),
            Invariant::new("sender is not the recipient", move || {
                sender_is_not_recipient
            }),
            Invariant::new("a pending transfer is not chained", move || {
                not_chained_yet || !is_pending
            }),
            Invariant::new("only a canceled transfer has a cancellation", move || {
                has_no_cancellation || is_canceled
            }),
            Invariant::new("cancellation has the nonce of the transfer", move || {
                cancellation_has_the_nonce_of_the_transfer
            }),
            Invariant::new("cancellation comes from the sender", move || {
                cancellation_comes_from_the_sender
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

    /// Canceled in MetaMask: the cancellation goes to the chain instead of this transfer.
    pub fn replace_by(self, cancellation: SignedTransaction) -> EnforcementResult<Self> {
        Self {
            status: TransferStatus::Canceled,
            cancellation: Some(cancellation),
            ..self
        }
        .validate()
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
