use crate::domain::services::transaction_hash::transaction_hash;
use crate::domain::value_objects::address::Address;
use crate::domain::value_objects::tx_hash::TxHash;
use alloy::consensus::{Signed, Transaction, TxEip1559, TxEnvelope};
use alloy::eips::eip2718::Decodable2718;
use alloy::primitives::hex;
use cerne::domain::{DomainError, EnforcementResult, Invariant, Invariants, ValueObject};
use serde::{Deserialize, Serialize};

/// The chain id of the RDE chain: the one configured in MetaMask.
pub const RDE_CHAIN_ID: u64 = 8808;

/// A transaction signed in the wallet of the sender (MetaMask), as `eth_sendRawTransaction` receives it:
/// `0x` + the EIP-2718 bytes of an EIP-1559 transaction.
///
/// Nobody sends the sender: it comes out of the signature. Everything else is read from the bytes, and the bytes
/// stay as they are, because they are what goes to the chain.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct SignedTransaction {
    raw: String,
    tx_hash: TxHash,
    sender: Address,
    recipient: Address,
    amount: u128,
    nonce: u64,
    gas_limit: u64,
    max_fee_per_gas: u128,
}

impl ValueObject for SignedTransaction {
    type Props = String;

    fn new(raw: String) -> EnforcementResult<Self> {
        let raw = raw.to_lowercase();
        let bytes = hex::decode(&raw).unwrap_or_default();
        let decoded = eip1559_transaction(&bytes);
        let transaction = decoded.as_ref().map(Signed::tx);
        let sender = decoded
            .as_ref()
            .and_then(|signed| signed.recover_signer().ok());
        let recipient = transaction.and_then(|tx| tx.to());
        let amount = transaction.and_then(|tx| u128::try_from(tx.value).ok());

        // Bytes that are not a transaction break only the first invariant: the others have nothing to look at.
        let is_an_eip1559_transaction = transaction.is_some();
        let signature_recovers_the_sender = transaction.is_none() || sender.is_some();
        let is_for_the_rde_chain = transaction.is_none_or(|tx| tx.chain_id == RDE_CHAIN_ID);
        let pays_a_wallet = transaction.is_none() || recipient.is_some();
        let carries_no_data = transaction.is_none_or(|tx| tx.input.is_empty());
        let amount_fits_in_128_bits = transaction.is_none() || amount.is_some();

        Invariants::new(vec![
            Invariant::new("signed transaction is EIP-1559", move || {
                is_an_eip1559_transaction
            }),
            Invariant::new("signature recovers the sender", move || {
                signature_recovers_the_sender
            }),
            Invariant::new("transaction is for the RDE chain", move || {
                is_for_the_rde_chain
            }),
            Invariant::new("transaction pays a wallet", move || pays_a_wallet),
            Invariant::new("transaction carries no data", move || carries_no_data),
            Invariant::new("amount fits in 128 bits", move || amount_fits_in_128_bits),
        ])
        .enforce()?;

        let (Some(transaction), Some(sender), Some(recipient), Some(amount)) =
            (transaction, sender, recipient, amount)
        else {
            unreachable!("the invariants above hold")
        };

        Ok(Self {
            tx_hash: TxHash::new(transaction_hash(&bytes))?,
            sender: Address::new(sender.to_string())?,
            recipient: Address::new(recipient.to_string())?,
            amount,
            nonce: transaction.nonce,
            gas_limit: transaction.gas_limit,
            max_fee_per_gas: transaction.max_fee_per_gas,
            raw,
        })
    }
}

/// The bytes as an EIP-1559 transaction, or `None` if they are anything else.
fn eip1559_transaction(bytes: &[u8]) -> Option<Signed<TxEip1559>> {
    match TxEnvelope::decode_2718(&mut &bytes[..]).ok()? {
        TxEnvelope::Eip1559(signed) => Some(signed),
        _ => None,
    }
}

impl SignedTransaction {
    /// The id of the transfer: the hash the chain and MetaMask know the transaction by.
    pub fn tx_hash(&self) -> &TxHash {
        &self.tx_hash
    }

    pub fn sender(&self) -> &Address {
        &self.sender
    }

    pub fn recipient(&self) -> &Address {
        &self.recipient
    }

    /// In wei: 1 RDEC is 10^18 wei.
    pub fn amount(&self) -> u128 {
        self.amount
    }

    pub fn nonce(&self) -> u64 {
        self.nonce
    }

    pub fn gas_limit(&self) -> u64 {
        self.gas_limit
    }

    /// In wei per unit of gas.
    pub fn max_fee_per_gas(&self) -> u128 {
        self.max_fee_per_gas
    }
}

impl TryFrom<String> for SignedTransaction {
    type Error = DomainError;

    fn try_from(raw: String) -> EnforcementResult<Self> {
        Self::new(raw)
    }
}

impl From<SignedTransaction> for String {
    fn from(signed_transaction: SignedTransaction) -> String {
        signed_transaction.raw
    }
}

impl std::fmt::Display for SignedTransaction {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str(&self.raw)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::services::units::WEI_PER_RDEC;
    use crate::infrastructure::local_wallet::LocalWallet;

    const ALICE_PRIVATE_KEY: &str =
        "0x0000000000000000000000000000000000000000000000000000000000000001";
    const BOB_PRIVATE_KEY: &str =
        "0x0000000000000000000000000000000000000000000000000000000000000002";

    #[test]
    fn the_sender_comes_out_of_the_signature() {
        let alice = LocalWallet::new(ALICE_PRIVATE_KEY);
        let bob = LocalWallet::new(BOB_PRIVATE_KEY);

        let signed_transaction = alice.sign_transfer(bob.address(), 100 * WEI_PER_RDEC, 0);

        assert_eq!(signed_transaction.sender(), alice.address());
        assert_eq!(signed_transaction.recipient(), bob.address());
        assert_eq!(signed_transaction.amount(), 100 * WEI_PER_RDEC);
    }

    #[test]
    fn the_tx_hash_is_the_one_the_chain_computes() {
        let alice = LocalWallet::new(ALICE_PRIVATE_KEY);
        let bob = LocalWallet::new(BOB_PRIVATE_KEY);

        let signed_transaction = alice.sign_transfer(bob.address(), 1, 0);

        let bytes = hex::decode(signed_transaction.to_string()).unwrap();
        let chain_hash = eip1559_transaction(&bytes).unwrap().hash().to_string();

        assert_eq!(signed_transaction.tx_hash().to_string(), chain_hash);
    }

    #[test]
    fn bytes_that_are_not_a_transaction_break_only_the_first_invariant() {
        assert_eq!(
            SignedTransaction::new("0x1234".into()),
            Err(DomainError::Violations(vec![
                "signed transaction is EIP-1559"
            ]))
        );
    }
}
