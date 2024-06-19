use std::sync::Arc;

use crate::error::AuthError;
use axum::{
    extract::{Request, State},
    http::request::Parts,
    middleware::Next,
    response::{IntoResponse, Response},
};
use axum_auth::{AuthBearer, AuthBearerCustom};
use axum_core::RequestExt;

use axum_jwt_auth::{Claims, Decoder, JwtDecoder, LocalDecoder};
use jsonwebtoken::{errors::ErrorKind, Algorithm, DecodingKey, TokenData, Validation};
use phylax_tracing::tracing::debug;
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::ApiState;

#[derive(Debug, Serialize, Deserialize)]
pub struct UserClaim {
    iat: u64,
    exp: u64,
}

/// Implementing Display for UserClaim
///
/// # Returns
///
/// * `fmt::Result` - Result of formatting
impl fmt::Display for UserClaim {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "UserClaim[ iat: {}, exp: {} ]", self.iat, self.exp)
    }
}

/// Function to create a JWT decoder
///
/// # Arguments
///
/// * `key` - A value that can be converted into bytes
///
/// # Returns
///
/// * `Decoder` - A JWT decoder
pub fn jwt_decoder<T: AsRef<[u8]>>(key: T) -> Decoder {
    let keys = vec![DecodingKey::from_secret(key.as_ref())];
    let validation = Validation::new(Algorithm::HS256);
    let decoder: Decoder = LocalDecoder::new(keys, validation).into();
    decoder
}

/// Middleware function for authentication
///
/// # Arguments
///
/// * `State(state)` - State of the API
/// * `request` - Incoming request
/// * `next` - Next middleware in the chain
///
/// # Returns
///
/// * `Response` - Response after processing the request
pub async fn auth_middleware(
    State(state): State<Arc<ApiState>>,
    mut request: Request,
    next: Next,
) -> Response {
    let mut parts: Parts = request.extract_parts().await.unwrap();
    let auth: Result<AuthBearer, axum_auth::Rejection> =
        AuthBearer::decode_request_parts(&mut parts);
    if auth.is_err() {
        tracing::warn!(error = %AuthError::MissingToken, "API Request Auth Error");
        return AuthError::MissingToken.into_response();
    }
    let token = auth.unwrap().0;
    let token_data: Result<TokenData<Claims<UserClaim>>, AuthError> =
        state.jwt_decoder.as_ref().unwrap().decode(token.as_str()).map_err(|e| match e {
            axum_jwt_auth::Error::Jwt(e) => match e.kind() {
                ErrorKind::ExpiredSignature => AuthError::ExpiredToken,
                ErrorKind::InvalidSignature => AuthError::InvalidSignature,
                _ => AuthError::InvalidToken,
            },
            _ => AuthError::InternalError,
        });
    if let Err(e) = token_data {
        tracing::warn!(error = %e, "API Request Auth Error");
        return e.into_response();
    }
    let claim = token_data.unwrap().claims.0;
    debug!(%claim, "JWT Authenticated");
    next.run(request).await
}
#[cfg(test)]
mod tests {
    use crate::{internal_bus::ApiInternalBus, TaskStates};

    use super::*;
    use axum::{
        http::{Request, StatusCode},
        middleware::{self},
        routing::get,
        Router,
    };
    use axum_core::body::Body;
    use jsonwebtoken::{encode, EncodingKey, Header};
    use phylax_common::metrics::MetricsRegistry;
    use phylax_config::PhConfig;
    use std::sync::Arc;
    use tower::ServiceExt; // for `call`, `oneshot`, and `ready`

    fn api_state(secret: &str) -> Arc<ApiState> {
        let config = Arc::new(PhConfig::default());
        Arc::new(ApiState {
            jwt_decoder: Some(jwt_decoder(secret)),
            config,
            metrics_registry: MetricsRegistry::new(),
            tasks_state: TaskStates::new(),
            internal_bus: ApiInternalBus::new(),
        })
    }

    #[test]
    fn test_jwt_decoder() {
        let secret = "secret";
        let gen_decoder = jwt_decoder(secret);
        let claims = UserClaim { iat: 0, exp: 10000000000 };
        let token =
            encode(&Header::default(), &claims, &EncodingKey::from_secret(secret.as_bytes()))
                .unwrap();
        let res: TokenData<UserClaim> = gen_decoder.decode(&token).unwrap();
        assert_eq!(res.claims.iat, claims.iat);
        assert_eq!(res.claims.exp, claims.exp);
    }

    #[tokio::test]
    async fn test_auth_middleware_accepts_correct_token() {
        let secret = "secret";
        let state = api_state(secret);

        // Create a valid token
        let claims = UserClaim { iat: 0, exp: 10000000000 };
        let token =
            encode(&Header::default(), &claims, &EncodingKey::from_secret(secret.as_bytes()))
                .unwrap();

        // Create a request with the token
        let correct_request = Request::builder()
            .uri("/")
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();

        // Correct token

        let middleware = middleware::from_fn_with_state(state.clone(), auth_middleware);
        let app = Router::new().route("/", get(|| async { "Hello, World!" })).layer(middleware);
        let response = app.oneshot(correct_request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
    #[tokio::test]
    async fn test_auth_middleware_rejects_incorrect_token() {
        let secret = "secret";
        let state = api_state(secret);

        // Create a valid token
        let claims = UserClaim { iat: 0, exp: 10000000000 };
        let token =
            encode(&Header::default(), &claims, &EncodingKey::from_secret(secret.as_bytes()))
                .unwrap();
        let wrong_request = Request::builder()
            .uri("/")
            .header("Authorization", format!("Bearer GARBAGE{token}"))
            .body(Body::empty())
            .unwrap();

        // Incorrect token

        let middleware = middleware::from_fn_with_state(state, auth_middleware);
        let app = Router::new().route("/", get(|| async { "Hello, World!" })).layer(middleware);
        let response = app.oneshot(wrong_request).await.unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}
