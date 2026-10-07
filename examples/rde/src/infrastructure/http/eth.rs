//! The Ethereum JSON-RPC that MetaMask calls to read the chain (`docs/METAMASK.md`). These are not commands nor
//! queries of the rde: each method answers from the `Blockchain` port, in the format of an Ethereum node.
//!
//! Quantities (balances, nonces, block numbers) go in hex with `0x`; the params come by position (an array).

use crate::application::commands::create_transfer::CreateTransferCommand;
use crate::application::commands::send_cancellation::SendCancellationCommand;
use crate::application::ports::blockchain::{Block, Receipt};
use crate::application::queries::received_transaction::ReceivedTransactionQuery;
use crate::domain::value_objects::address::Address;
use crate::domain::value_objects::signed_transaction::{RDE_CHAIN_ID, SignedTransaction};
use crate::domain::value_objects::tx_hash::TxHash;
use crate::ports::Ports;
use cerne::application::Query;
use cerne::http::jsonrpc::{ErrorObject, Methods};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

/// The gas of a transfer between two wallets, the only transaction the RDE chain takes.
const TRANSFER_GAS: u64 = 21_000;

// --- The chain ---------------------------------------------------------------

/// `eth_chainId`: MetaMask calls it when the network is added, and refuses the network if it differs.
pub fn chain_id() -> Result<Value, ErrorObject> {
    Ok(json!(quantity(RDE_CHAIN_ID as u128)))
}

/// `net_version`: the same chain id, in decimal and as text.
pub fn net_version() -> Result<Value, ErrorObject> {
    Ok(json!(RDE_CHAIN_ID.to_string()))
}

/// `eth_blockNumber`: MetaMask calls it every 20 seconds, and reads again what depends on it when it changes.
pub async fn block_number(ports: &Ports) -> Result<Value, ErrorObject> {
    let block_number = ports.blockchain.block_number().await?;

    Ok(json!(quantity(block_number as u128)))
}

/// `eth_getBlockByNumber` with `[block, full]`. The transactions always come as hashes.
pub async fn block_by_number(ports: &Ports, params: Value) -> Result<Value, ErrorObject> {
    let (block, _full): (String, bool) = positional(params)?;

    let number = match block.as_str() {
        "earliest" => 0,
        "latest" | "pending" | "safe" | "finalized" => ports.blockchain.block_number().await?,
        hex => u64::from_str_radix(hex.trim_start_matches("0x"), 16)
            .map_err(|_| ErrorObject::invalid_params(format!("{hex} is not a block")))?,
    };

    let block = ports.blockchain.block(number).await?;

    Ok(block.map_or(Value::Null, block_json))
}

/// `eth_getBlockByHash` with `[hash, full]`: MetaMask asks for the block of a receipt with `status: "0x1"`, and
/// confirms the transaction only if it gets it.
pub async fn block_by_hash(ports: &Ports, params: Value) -> Result<Value, ErrorObject> {
    let (hash, _full): (String, bool) = positional(params)?;

    let block = ports.blockchain.block_by_hash(&hash).await?;

    Ok(block.map_or(Value::Null, block_json))
}

/// `eth_gasPrice`, in wei.
pub async fn gas_price(ports: &Ports) -> Result<Value, ErrorObject> {
    let gas_price = ports.blockchain.gas_price().await?;

    Ok(json!(quantity(gas_price)))
}

/// `eth_estimateGas`: every transaction on the RDE chain is a transfer between two wallets.
pub fn estimate_gas() -> Result<Value, ErrorObject> {
    Ok(json!(quantity(TRANSFER_GAS as u128)))
}

// --- Wallets -----------------------------------------------------------------

/// `eth_getBalance` with `[address, block]`, in wei. The balance is always the latest one.
pub async fn balance(ports: &Ports, params: Value) -> Result<Value, ErrorObject> {
    let (wallet, _block): (Address, Value) = positional(params)?;

    let balance = ports.blockchain.available_rdec(&wallet).await?;

    Ok(json!(quantity(balance)))
}

/// `eth_getTransactionCount` with `[address, block]`: the nonce the next transaction of the wallet must carry.
pub async fn transaction_count(ports: &Ports, params: Value) -> Result<Value, ErrorObject> {
    let (wallet, _block): (Address, Value) = positional(params)?;

    let next_nonce = ports.blockchain.next_nonce(&wallet).await?;

    Ok(json!(quantity(next_nonce as u128)))
}

/// `eth_getCode`: the RDE chain has no contracts, so every address is a wallet.
pub fn code() -> Result<Value, ErrorObject> {
    Ok(json!("0x"))
}

/// `eth_call`: with no contracts, every call returns nothing. MetaMask asks before a send whether the recipient
/// is a token (`symbol`, `decimals`, `balanceOf`), and nothing means it is not.
pub fn call() -> Result<Value, ErrorObject> {
    Ok(json!("0x"))
}

// --- Transactions --------------------------------------------------------------

/// `eth_sendRawTransaction` with `[signed_transaction]`: what MetaMask sends when the sender clicks "Send", or "Cancel"
/// on a pending transfer. Both answer with the hash MetaMask follows from then on.
///
/// - A transaction the rde already has comes back with its hash, as on a node: "Speed up" on a cancellation sends
///   the same bytes again.
/// - Nothing to the sender itself is a cancellation: the `SendCancellationCommand`.
/// - Anything else is a new transfer: the `CreateTransferCommand`. "Speed up" on a pending transfer lands here, and
///   the rule "sender has no open transfer" refuses it.
pub async fn send_raw_transaction(
    ports: &Ports,
    methods: &Methods<'_, Ports>,
    params: Value,
) -> Result<Value, ErrorObject> {
    let (signed_transaction,): (SignedTransaction,) = positional(params.clone())?;

    let received_transaction_query = ReceivedTransactionQuery {
        tx_hash: signed_transaction.tx_hash().clone(),
    };

    let received_transaction = received_transaction_query.execute(ports).await?;

    if received_transaction.received {
        return Ok(json!(received_transaction.tx_hash));
    }

    if signed_transaction.is_a_cancellation() {
        methods.command::<SendCancellationCommand>(params).await
    } else {
        methods.command::<CreateTransferCommand>(params).await
    }
}

/// `eth_getTransactionReceipt` with `[tx_hash]`: `null` while the transfer is in no block. MetaMask asks every 3
/// seconds after a send, then on every new block.
pub async fn receipt(ports: &Ports, params: Value) -> Result<Value, ErrorObject> {
    let (tx_hash,): (TxHash,) = positional(params)?;

    let receipt = ports.blockchain.receipt(&tx_hash).await?;

    Ok(receipt.map_or(Value::Null, receipt_json))
}

/// `eth_getTransactionByHash`: always `null`. MetaMask asks it next to the receipt, and only the receipt matters: a
/// canceled transfer has no receipt, and MetaMask shows it as failed once the cancellation is in a block.
pub fn transaction_by_hash() -> Result<Value, ErrorObject> {
    Ok(Value::Null)
}

// --- Formats -----------------------------------------------------------------

fn quantity(value: u128) -> String {
    format!("0x{value:x}")
}

fn zeros(bytes: usize) -> String {
    format!("0x{}", "00".repeat(bytes))
}

fn block_json(block: Block) -> Value {
    json!({
        "number": quantity(block.number as u128),
        "hash": block.hash,
        "parentHash": block.parent_hash,
        "timestamp": quantity(block.timestamp as u128),
        "baseFeePerGas": quantity(block.base_fee_per_gas),
        "gasLimit": quantity(30_000_000),
        "gasUsed": quantity(TRANSFER_GAS as u128 * block.transactions.len() as u128),
        "transactions": block.transactions,
        "miner": zeros(20),
        "difficulty": "0x0",
        "totalDifficulty": "0x0",
        "nonce": zeros(8),
        "extraData": "0x",
        "size": "0x200",
        "uncles": [],
        "logsBloom": zeros(256),
        "mixHash": zeros(32),
        "sha3Uncles": zeros(32),
        "stateRoot": zeros(32),
        "receiptsRoot": zeros(32),
        "transactionsRoot": zeros(32),
    })
}

fn receipt_json(receipt: Receipt) -> Value {
    json!({
        "transactionHash": receipt.tx_hash,
        "transactionIndex": "0x0",
        "blockHash": receipt.block_hash,
        "blockNumber": quantity(receipt.block_number as u128),
        "from": receipt.sender,
        "to": receipt.recipient,
        "status": if receipt.succeeded { "0x1" } else { "0x0" },
        "gasUsed": quantity(receipt.gas_used as u128),
        "cumulativeGasUsed": quantity(receipt.gas_used as u128),
        "effectiveGasPrice": quantity(receipt.gas_price),
        "type": "0x2",
        "contractAddress": null,
        "logs": [],
        "logsBloom": zeros(256),
    })
}

// --- Params ------------------------------------------------------------------

/// The params by position, read into a tuple: `[address, block]` into `(Address, Value)`.
fn positional<T: DeserializeOwned>(params: Value) -> Result<T, ErrorObject> {
    serde_json::from_value(params).map_err(ErrorObject::invalid_params)
}
