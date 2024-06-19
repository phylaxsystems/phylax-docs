use std::io;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    BoxError,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("The server encountered an error: {source}")]
    Server {
        #[from]
        source: axum::Error,
    },
    #[error("The API is not aware of the task [TASK: {0}]")]
    UnknownTaskStatus(String),
    #[error("API doesn't recognize the task's state: {0}")]
    InvalidTaskState(String),
    #[error("API failed to bind port")]
    PortBind {
        #[from]
        source: io::Error,
    },
}

#[derive(Debug, Error)]
pub enum ApiInternalBusError<T> {
    #[error("The bus encountered an error when sending a messsage")]
    Send {
        #[from]
        source: flume::SendError<T>,
    },
    #[error("The bus encountered an error when receiving a message")]
    Receive {
        #[from]
        source: flume::RecvError,
    },
}

/// Error type which implements `IntoResponse`
/// Taken from https://github.com/knarkzel/axum-error but didn't want
/// to include a whole crate for one type.
#[derive(Debug)]
pub struct ConvertibleError(pub color_eyre::Report);

impl<E: Into<color_eyre::Report>> From<E> for ConvertibleError {
    fn from(error: E) -> Self {
        ConvertibleError(error.into())
    }
}

impl IntoResponse for ConvertibleError {
    fn into_response(self) -> Response {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("{:?}", self.0)).into_response()
    }
}

impl From<ConvertibleError> for BoxError {
    fn from(error: ConvertibleError) -> Self {
        error.0.into()
    }
}

// Result type
pub type EyreResult<T, E = ConvertibleError> = std::result::Result<T, E>;

/// Enum representing various authentication errors
#[derive(thiserror::Error, Debug)]
pub enum AuthError {
    /// Error indicating that the provided token is invalid
    #[error("Invalid token provided")]
    InvalidToken,

    /// Error indicating that the token is missing
    #[error("Token is missing")]
    MissingToken,

    /// Error indicating that the provided token has expired
    #[error("Provided token has expired")]
    ExpiredToken,

    /// Error indicating that the provided token has an invalid signature
    #[error("Invalid signature in the provided token")]
    InvalidSignature,

    /// Error indicating an internal error during authentication
    #[error("Internal error during authentication")]
    InternalError,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, msg) = match self {
            AuthError::InvalidToken => (StatusCode::UNAUTHORIZED, "Invalid token"),
            AuthError::MissingToken => (StatusCode::UNAUTHORIZED, "Missing token"),
            AuthError::ExpiredToken => (StatusCode::UNAUTHORIZED, "Expired token"),
            AuthError::InvalidSignature => (StatusCode::UNAUTHORIZED, "Invalid signature"),
            AuthError::InternalError => (StatusCode::INTERNAL_SERVER_ERROR, "Internal error"),
        };

        (status, msg).into_response()
    }
}
