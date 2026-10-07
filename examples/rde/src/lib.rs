//! The transfer lanes of the RDE Blockchain Event Storming:
//!
//! - Sender → `CreateTransferCommand` → business rules → `Transfer` (Pending) → `TransferCreated`
//! - Recipient → `AcceptTransferCommand` → `Transfer` (Accepted) → `TransferAcceptedByRecipient`
//!   → policy "whenever a transfer is accepted, chain it" → `ChainAcceptedTransferCommand` → Blockchain → `TransactionChained`
//! - Recipient → `RejectTransferCommand` → `Transfer` (Rejected) → `TransferRejectedByRecipient`
//! - Sender → `CancelTransferCommand` → `Transfer` (Canceled) → `TransferCanceledBySender`

pub mod application;
pub mod domain;
pub mod infrastructure;
pub mod ports;
