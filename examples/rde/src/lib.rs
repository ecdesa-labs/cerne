//! The transfer lanes of the RDE Blockchain Event Storming. The sender uses MetaMask: the rde speaks the Ethereum
//! JSON-RPC, and a transfer arrives as the transaction MetaMask signed (`eth_sendRawTransaction`).
//!
//! - Sender → `CreateTransferCommand` → business rules → `Transfer` (Pending) → `TransferCreated`
//!   → policy "whenever a transfer is created, notify the recipient" → `NotifyRecipientCommand` → `RecipientNotified`
//! - Recipient → `AcceptTransferCommand` → `Transfer` (Accepted) → `TransferAcceptedByRecipient`
//!   → policy "whenever a transfer is accepted, chain it" → `ChainAcceptedTransferCommand` → Blockchain → `TransactionChained`
//! - Recipient → `RejectTransferCommand` → `Transfer` (Rejected) → `TransferRejectedByRecipient`
//!   → policy "whenever a transfer is rejected, chain it as failed" → `ChainFailedTransferCommand` → Blockchain
//!   → `FailedTransactionChained`
//! - Sender → `CancelTransferCommand` → `Transfer` (Canceled) → `TransferCanceledBySender`
//!   → policy "whenever a transfer is canceled, chain it as failed" → `ChainFailedTransferCommand`

pub mod application;
pub mod domain;
pub mod infrastructure;
pub mod ports;
