use crate::domain::value_objects::address::Address;
use crate::domain::value_objects::signed_transaction::{RDE_CHAIN_ID, SignedTransaction};
use alloy::consensus::{SignableTransaction, TxEip1559, TxEnvelope};
use alloy::eips::eip2718::Encodable2718;
use alloy::network::TxSignerSync;
use alloy::primitives::{TxKind, U256, hex};
use alloy::signers::local::PrivateKeySigner;
use cerne::domain::ValueObject;

/// 1 gwei, what MetaMask paid in the capture (`docs/METAMASK.md`).
const MAX_FEE_PER_GAS: u128 = 1_000_000_000;

/// 1.1 gwei: a cancellation or a speed-up pays 10% more than the transaction it replaces.
const REPLACEMENT_MAX_FEE_PER_GAS: u128 = 1_100_000_000;

/// A stand-in for MetaMask: holds a private key and signs transfers on the RDE chain, the same way MetaMask does
/// (EIP-1559, 21000 of gas at 1 gwei). The rde never sees the key, only the signed transaction.
pub struct LocalWallet {
    signer: PrivateKeySigner,
    address: Address,
}

impl LocalWallet {
    /// `private_key` is `0x` + 64 hex digits. Panics on anything else: the keys are fixed in the examples and tests.
    pub fn new(private_key: &str) -> Self {
        let signer: PrivateKeySigner = private_key.parse().expect("a valid private key");
        let address = Address::new(signer.address().to_string()).expect("an address");

        Self { signer, address }
    }

    pub fn address(&self) -> &Address {
        &self.address
    }

    /// Signs `amount` wei to `recipient`. `nonce` must be the next one of this wallet on the chain.
    pub fn sign_transfer(
        &self,
        recipient: &Address,
        amount: u128,
        nonce: u64,
    ) -> SignedTransaction {
        self.sign(recipient, amount, nonce, MAX_FEE_PER_GAS)
    }

    /// What MetaMask signs on "Cancel": nothing to this wallet itself, with the `nonce` of the pending transfer and a
    /// fee 10% higher.
    pub fn sign_cancellation(&self, nonce: u64) -> SignedTransaction {
        self.sign(&self.address, 0, nonce, REPLACEMENT_MAX_FEE_PER_GAS)
    }

    /// What MetaMask signs on "Speed up": the same transfer, with the same `nonce` and a fee 10% higher.
    pub fn sign_speed_up(
        &self,
        recipient: &Address,
        amount: u128,
        nonce: u64,
    ) -> SignedTransaction {
        self.sign(recipient, amount, nonce, REPLACEMENT_MAX_FEE_PER_GAS)
    }

    fn sign(
        &self,
        recipient: &Address,
        amount: u128,
        nonce: u64,
        max_fee_per_gas: u128,
    ) -> SignedTransaction {
        let recipient = recipient.to_string().parse().expect("an address");

        let mut transaction = TxEip1559 {
            chain_id: RDE_CHAIN_ID,
            nonce,
            gas_limit: 21_000,
            max_fee_per_gas,
            max_priority_fee_per_gas: max_fee_per_gas,
            to: TxKind::Call(recipient),
            value: U256::from(amount),
            ..Default::default()
        };

        let signature = self
            .signer
            .sign_transaction_sync(&mut transaction)
            .expect("a signature");
        let envelope = TxEnvelope::from(transaction.into_signed(signature));

        SignedTransaction::new(hex::encode_prefixed(envelope.encoded_2718()))
            .expect("a transaction signed for the RDE chain")
    }
}
