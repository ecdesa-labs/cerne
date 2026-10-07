use crate::domain::value_objects::address::Address;
use crate::domain::value_objects::signed_transaction::{RDE_CHAIN_ID, SignedTransaction};
use alloy::consensus::{SignableTransaction, TxEip1559, TxEnvelope};
use alloy::eips::eip2718::Encodable2718;
use alloy::network::TxSignerSync;
use alloy::primitives::{TxKind, U256, hex};
use alloy::signers::local::PrivateKeySigner;
use cerne::domain::ValueObject;

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
        let recipient = recipient.to_string().parse().expect("an address");

        let mut transaction = TxEip1559 {
            chain_id: RDE_CHAIN_ID,
            nonce,
            gas_limit: 21_000,
            max_fee_per_gas: 1_000_000_000,
            max_priority_fee_per_gas: 1_000_000_000,
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
