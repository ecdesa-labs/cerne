use crate::application::commands::accept_transfer::AcceptTransferCommand;
use crate::application::commands::cancel_transfer::CancelTransferCommand;
use crate::application::commands::create_transfer::CreateTransferCommand;
use crate::application::commands::reject_transfer::RejectTransferCommand;
use crate::application::queries::pending_transfers::PendingTransfersQuery;
use crate::ports::Ports;
use axum::Json;
use axum::extract::State;
use cerne::http::jsonrpc::{Methods, Request, Response};
use std::sync::Arc;

/// One method per command or query an actor sends, named in snake_case: `create_transfer` reads its params into a
/// `CreateTransferCommand` and executes it in a transaction. The actor comes in the params until there is
/// authentication.
pub async fn rpc(State(ports): State<Arc<Ports>>, Json(request): Json<Request>) -> Json<Response> {
    let methods = Methods::new(ports.as_ref());

    let result = match request.method.as_str() {
        // --- Sender ------------------------------------------------------------
        "create_transfer" => {
            methods
                .command::<CreateTransferCommand>(request.params)
                .await
        }
        "cancel_transfer" => {
            methods
                .command::<CancelTransferCommand>(request.params)
                .await
        }

        // --- Recipient ---------------------------------------------------------
        "pending_transfers" => methods.query::<PendingTransfersQuery>(request.params).await,
        "accept_transfer" => {
            methods
                .command::<AcceptTransferCommand>(request.params)
                .await
        }
        "reject_transfer" => {
            methods
                .command::<RejectTransferCommand>(request.params)
                .await
        }

        method => methods.not_found(method),
    };

    Json(Response::new(request.id, result))
}
