//! HTTP with `axum` (feature `axum`): a project speaks REST or JSON-RPC, chosen in `cerne new --http rest|jsonrpc`.
//! The body of a request is the command (or the query) itself, read with `serde`.
//!
//! The three categories of [`Error`] become the answer:
//!
//! | Error | REST | JSON-RPC |
//! |---|---|---|
//! | `DomainError` | 422, with the violations | `-32001`, with the violations in `message` and in `data` |
//! | `ApplicationError::NotFound` | 404 | `-32004` |
//! | `InfrastructureError` | 500, without details | `-32603` (internal error), without details |

use crate::errors::{ApplicationError, DomainError, Error};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::{Value, json};

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let (status, body) = match &self {
            Error::Domain(DomainError::Violations(violations)) => {
                (StatusCode::UNPROCESSABLE_ENTITY, json!({ "error": "domain", "violations": violations }))
            }
            Error::Application(ApplicationError::NotFound(what)) => {
                (StatusCode::NOT_FOUND, json!({ "error": "not_found", "message": format!("{what} not found") }))
            }
            Error::Infrastructure(_) => (StatusCode::INTERNAL_SERVER_ERROR, json!({ "error": "infrastructure" })),
        };

        (status, axum::Json(body)).into_response()
    }
}

/// JSON-RPC 2.0 over a single `POST`: the method is the snake_case of the command or query (`place_order`).
///
/// The handler reads the raw body with [`Request::from_body`](jsonrpc::Request::from_body), so a body that is not JSON-RPC 2.0 still gets a
/// JSON-RPC answer:
///
/// | Body | Error |
/// |---|---|
/// | not JSON | `-32700` (parse error) |
/// | JSON without `method`, or `jsonrpc` other than `"2.0"` | `-32600` (invalid request) |
/// | `params` that is neither an array nor an object, or does not fit the command | `-32602` (invalid params) |
pub mod jsonrpc {
    use super::*;
    use crate::application::{Command, Query, TransactionalCompositionRoot};
    use serde::de::DeserializeOwned;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Deserialize)]
    pub struct Request {
        pub jsonrpc: String,
        pub method: String,
        #[serde(default)]
        pub params: Value,
        #[serde(default)]
        pub id: Value,
    }

    impl Request {
        /// Reads the raw body of the `POST`. On error, the answer goes back with `"id": null` (and HTTP 200).
        pub fn from_body(body: &[u8]) -> Result<Self, ErrorObject> {
            let json: Value = serde_json::from_slice(body).map_err(ErrorObject::parse_error)?;

            let request: Request = serde_json::from_value(json).map_err(ErrorObject::invalid_request)?;

            let speaks_jsonrpc_2 = request.jsonrpc == "2.0";

            if !speaks_jsonrpc_2 {
                return Err(ErrorObject {
                    code: -32600,
                    message: format!("invalid request: jsonrpc must be \"2.0\", not {:?}", request.jsonrpc),
                    data: None,
                });
            }

            Ok(request)
        }
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

        /// The body is not JSON.
        pub fn parse_error(error: serde_json::Error) -> Self {
            Self {
                code: -32700,
                message: format!("parse error: {error}"),
                data: None,
            }
        }

        /// The body is JSON, but not a JSON-RPC request: no `method`, or no `jsonrpc`.
        pub fn invalid_request(error: serde_json::Error) -> Self {
            Self {
                code: -32600,
                message: format!("invalid request: {error}"),
                data: None,
            }
        }

        /// The params do not fit what the method reads.
        pub fn invalid_params(error: impl std::fmt::Display) -> Self {
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
                    // A wallet like MetaMask shows only the message: the violations go in it too.
                    message: format!("the domain refused the request: {}", violations.join(", ")),
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
    /// let methods = Methods::new(&composition_root);
    ///
    /// let result = match request.method.as_str() {
    ///     "place_order" => methods.command::<PlaceOrderCommand>(request.params).await,
    ///     "placed_orders" => methods.query::<PlacedOrdersQuery>(request.params).await,
    ///     method => methods.not_found(method),
    /// };
    /// ```
    ///
    /// The `params` come by name (an object) or by position (an array, like MetaMask sends them), as JSON-RPC 2.0
    /// allows. By position, the array follows the order of the fields of the command: that order is part of
    /// the API, and swapping two fields of the same type breaks the clients without a compilation error.
    pub struct Methods<'p, CompositionRoot> {
        composition_root: &'p CompositionRoot,
    }

    impl<'p, CompositionRoot: TransactionalCompositionRoot> Methods<'p, CompositionRoot> {
        pub fn new(composition_root: &'p CompositionRoot) -> Self {
            Self { composition_root }
        }

        /// The last arm of the `match`: no command or query has this name.
        pub fn not_found(&self, method: &str) -> Result<Value, ErrorObject> {
            Err(ErrorObject::method_not_found(method))
        }

        /// Reads the command from `params` and executes it in a transaction; the result is its output.
        pub async fn command<C>(&self, params: Value) -> Result<Value, ErrorObject>
        where
            C: Command<CompositionRoot> + DeserializeOwned + 'static,
            C::Output: Serialize,
        {
            let command: C = read_params(params)?;

            let output = self
                .composition_root
                .execute_in_transaction(command)
                .await?;

            serde_json::to_value(output).map_err(|error| infrastructure(error).into())
        }

        /// Reads the query from `params` and executes it; the result is its read model.
        pub async fn query<Q>(&self, params: Value) -> Result<Value, ErrorObject>
        where
            Q: Query<CompositionRoot> + DeserializeOwned,
            Q::ReadModel: Serialize,
        {
            let query: Q = read_params(params)?;

            let read_model = query.execute(self.composition_root).await?;

            serde_json::to_value(read_model).map_err(|error| infrastructure(error).into())
        }
    }

    /// By name (an object), by position (an array, in the order of the fields) or absent; anything else is `-32602`.
    fn read_params<T: DeserializeOwned>(params: Value) -> Result<T, ErrorObject> {
        let params_are_by_name_or_by_position = matches!(params, Value::Object(_) | Value::Array(_) | Value::Null);

        if !params_are_by_name_or_by_position {
            return Err(ErrorObject::invalid_params("params must be an array (by position) or an object (by name)"));
        }

        serde_json::from_value(params).map_err(ErrorObject::invalid_params)
    }

    fn infrastructure(error: serde_json::Error) -> Error {
        crate::errors::InfrastructureError::from(anyhow::Error::from(error)).into()
    }
}
