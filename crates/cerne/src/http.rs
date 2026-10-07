//! HTTP with `axum` (feature `axum`): a project speaks REST or JSON-RPC, chosen in `cerne new --http rest|jsonrpc`
//! (D33). The body of a request is the command (or the query) itself, read with `serde`.
//!
//! The three categories of [`Error`] become the answer:
//!
//! | Error | REST | JSON-RPC |
//! |---|---|---|
//! | `DomainError` | 422, with the violations | `-32001`, with the violations in `data` |
//! | `ApplicationError::NotFound` | 404 | `-32004` |
//! | `InfrastructureError` | 500, without details | `-32603` (internal error), without details |

use crate::errors::{ApplicationError, DomainError, Error};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::{Value, json};

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let (status, body) = match &self {
            Error::Domain(DomainError::Violations(violations)) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                json!({ "error": "domain", "violations": violations }),
            ),
            Error::Application(ApplicationError::NotFound(what)) => (
                StatusCode::NOT_FOUND,
                json!({ "error": "not_found", "message": format!("{what} not found") }),
            ),
            Error::Infrastructure(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({ "error": "infrastructure" }),
            ),
        };

        (status, axum::Json(body)).into_response()
    }
}

/// JSON-RPC 2.0 over a single `POST`: the method is the snake_case of the command or query (`create_transfer`).
pub mod jsonrpc {
    use super::*;
    use crate::application::{Command, Query, TransactionalPorts};
    use serde::de::DeserializeOwned;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Deserialize)]
    pub struct Request {
        pub method: String,
        #[serde(default)]
        pub params: Value,
        #[serde(default)]
        pub id: Value,
    }

    #[derive(Debug, Serialize)]
    pub struct Response {
        jsonrpc: &'static str,
        #[serde(skip_serializing_if = "Option::is_none")]
        result: Option<Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<ErrorObject>,
        id: Value,
    }

    impl Response {
        pub fn new(id: Value, result: Result<Value, ErrorObject>) -> Self {
            let (result, error) = match result {
                Ok(result) => (Some(result), None),
                Err(error) => (None, Some(error)),
            };

            Self {
                jsonrpc: "2.0",
                result,
                error,
                id,
            }
        }
    }

    #[derive(Debug, Clone, PartialEq, Serialize)]
    pub struct ErrorObject {
        pub code: i64,
        pub message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub data: Option<Value>,
    }

    impl ErrorObject {
        pub fn method_not_found(method: &str) -> Self {
            Self {
                code: -32601,
                message: format!("method {method} not found"),
                data: None,
            }
        }

        /// The params do not fit what the method reads.
        pub fn invalid_params(error: serde_json::Error) -> Self {
            Self {
                code: -32602,
                message: format!("invalid params: {error}"),
                data: None,
            }
        }
    }

    impl From<Error> for ErrorObject {
        fn from(error: Error) -> Self {
            match error {
                Error::Domain(DomainError::Violations(violations)) => Self {
                    code: -32001,
                    message: "the domain refused the request".into(),
                    data: Some(json!({ "violations": violations })),
                },
                Error::Application(ApplicationError::NotFound(what)) => Self {
                    code: -32004,
                    message: format!("{what} not found"),
                    data: None,
                },
                Error::Infrastructure(_) => Self {
                    code: -32603,
                    message: "internal error".into(),
                    data: None,
                },
            }
        }
    }

    /// The methods of a JSON-RPC endpoint: each arm of its `match` reads the params into a command or a query.
    ///
    /// ```ignore
    /// let methods = Methods::new(&ports);
    ///
    /// let result = match request.method.as_str() {
    ///     "create_transfer" => methods.command::<CreateTransferCommand>(request.params).await,
    ///     "pending_transfers" => methods.query::<PendingTransfersQuery>(request.params).await,
    ///     method => methods.not_found(method),
    /// };
    /// ```
    pub struct Methods<'p, Ports> {
        ports: &'p Ports,
    }

    impl<'p, Ports: TransactionalPorts> Methods<'p, Ports> {
        pub fn new(ports: &'p Ports) -> Self {
            Self { ports }
        }

        /// The last arm of the `match`: no command or query has this name.
        pub fn not_found(&self, method: &str) -> Result<Value, ErrorObject> {
            Err(ErrorObject::method_not_found(method))
        }

        /// Reads the command from `params` and executes it in a transaction; the result is its output.
        pub async fn command<C>(&self, params: Value) -> Result<Value, ErrorObject>
        where
            C: Command<Ports> + DeserializeOwned + 'static,
            C::Output: Serialize,
        {
            let command: C = serde_json::from_value(params).map_err(ErrorObject::invalid_params)?;

            let output = self.ports.execute_in_transaction(command).await?;

            serde_json::to_value(output).map_err(|error| infrastructure(error).into())
        }

        /// Reads the query from `params` and executes it; the result is its read model.
        pub async fn query<Q>(&self, params: Value) -> Result<Value, ErrorObject>
        where
            Q: Query<Ports> + DeserializeOwned,
            Q::ReadModel: Serialize,
        {
            let query: Q = serde_json::from_value(params).map_err(ErrorObject::invalid_params)?;

            let read_model = query.execute(self.ports).await?;

            serde_json::to_value(read_model).map_err(|error| infrastructure(error).into())
        }
    }

    fn infrastructure(error: serde_json::Error) -> Error {
        crate::errors::InfrastructureError::from(anyhow::Error::from(error)).into()
    }
}
