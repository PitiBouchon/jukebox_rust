use crate::login::error::AuthError;
use axum::async_trait;
use axum::body::Body;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::Request;
use axum_extra::extract::CookieJar;
use jsonwebtoken::errors::ErrorKind;
use jsonwebtoken::{decode, DecodingKey, EncodingKey, Validation};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

pub static KEYS: Lazy<Keys> = Lazy::new(|| Keys::new("top-secret-key".as_bytes()));

pub struct Keys {
    pub encoding: EncodingKey,
    decoding: DecodingKey,
}

impl Keys {
    fn new(secret: &[u8]) -> Self {
        Self {
            encoding: EncodingKey::from_secret(secret),
            decoding: DecodingKey::from_secret(secret),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthToken {
    pub username: String,
    // Expiration date
    pub exp: u64,
}

#[async_trait]
impl<S> FromRequestParts<S> for AuthToken {
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let jar = CookieJar::from_headers(&parts.headers);
        let token_cookie = jar
            .get("access_token")
            .ok_or(AuthError::MissingCredentials)?;
        let token_data =
            decode::<AuthToken>(token_cookie.value(), &KEYS.decoding, &Validation::default())
                .map_err(|err| match err.kind() {
                    ErrorKind::ExpiredSignature => AuthError::TokenExpired,
                    _ => AuthError::InvalidToken,
                })?;
        Ok(token_data.claims)
    }
}

impl AuthToken {
    pub async fn from_request(request: &Request<Body>) -> Result<Self, AuthError> {
        let jar = CookieJar::from_headers(request.headers());
        let token_cookie = jar
            .get("access_token")
            .ok_or(AuthError::MissingCredentials)?;
        let token_data =
            decode::<AuthToken>(token_cookie.value(), &KEYS.decoding, &Validation::default())
                .map_err(|_| AuthError::InvalidToken)?;
        Ok(token_data.claims)
    }
}
