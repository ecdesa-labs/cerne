use crate::application::commands::accept_transfer::AcceptTransferCommand;
use crate::application::commands::cancel_transfer::CancelTransferCommand;
use crate::application::commands::create_transfer::CreateTransferCommand;
use crate::application::commands::reject_transfer::RejectTransferCommand;
use crate::application::queries::pending_transfers::PendingTransfersQuery;
use crate::infrastructure::http::eth;
use crate::ports::Ports;
use axum::Json;
use axum::extract::State;
use cerne::http::jsonrpc::{Methods, Request, Response};
use std::sync::Arc;

/// One endpoint for two kinds of client:
///
/// - MetaMask speaks the Ethereum JSON-RPC: `eth_sendRawTransaction` is the `CreateTransferCommand`, and the other
///   `eth_*` methods read the chain (`eth.rs`).
/// - The rde methods, named in snake_case after their command or query: `accept_transfer` reads its params into an
///   `AcceptTransferCommand` and executes it in a transaction. The actor comes in the params until there is
///   authentication.
pub async fn rpc(State(ports): State<Arc<Ports>>, Json(request): Json<Request>) -> Json<Response> {
    let methods = Methods::new(ports.as_ref());
    let params = request.params;

    let result = match request.method.as_str() {
        // --- Sender, through MetaMask --------------------------------------------
        "eth_sendRawTransaction" => methods.command::<CreateTransferCommand>(params).await,

        // --- MetaMask reading the chain --------------------------------------------
        "eth_chainId" => eth::chain_id(),
        "net_version" => eth::net_version(),
        "eth_blockNumber" => eth::block_number(&ports).await,
        "eth_getBlockByNumber" => eth::block_by_number(&ports, params).await,
        "eth_gasPrice" => eth::gas_price(&ports).await,
        "eth_estimateGas" => eth::estimate_gas(),
        "eth_getBalance" => eth::balance(&ports, params).await,
        "eth_getTransactionCount" => eth::transaction_count(&ports, params).await,
        "eth_getCode" => eth::code(),
        "eth_call" => eth::call(),
        "eth_getTransactionReceipt" => eth::receipt(&ports, params).await,
        "eth_getTransactionByHash" => eth::transaction_by_hash(),

        // --- Sender ----------------------------------------------------------------
        "cancel_transfer" => methods.command::<CancelTransferCommand>(params).await,

        // --- Recipient -------------------------------------------------------------
        "pending_transfers" => methods.query::<PendingTransfersQuery>(params).await,
        "accept_transfer" => methods.command::<AcceptTransferCommand>(params).await,
        "reject_transfer" => methods.command::<RejectTransferCommand>(params).await,

        method => methods.not_found(method),
    };

    Json(Response::new(request.id, result))
}
