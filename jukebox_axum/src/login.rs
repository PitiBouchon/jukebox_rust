mod error;
pub mod jwt_token;

use crate::{sql, templates::login::LoginTemplate, templates::HtmlTemplate, AppState};
use axum::extract::{Form, State};
use axum::response::{IntoResponse, Redirect};
use axum_extra::extract::cookie::Cookie;
use axum_extra::extract::CookieJar;
use entity::user;
use error::AuthError;
use jsonwebtoken::{encode, Header};
use jwt_token::{AuthToken, KEYS};
use serde::Serialize;
use std::sync::Arc;
use std::time::UNIX_EPOCH;
use tracing::log;

pub const TOKEN_DURATION_SECONDS: u64 = 60;

#[derive(Serialize)]
pub struct AuthBody {
    pub access_token: String,
    pub token_type: String,
}

#[axum::debug_handler]
pub async fn authorize(
    State(state): State<Arc<super::AppState>>,
    jar: CookieJar, // TODO : change this to PrivateCookieJar
    Form(form): Form<user::Model>,
) -> Result<(CookieJar, Redirect), AuthError> {
    log::debug!("Post /login");
    if form.login.is_empty() || form.password.is_empty() {
        return Err(AuthError::MissingCredentials);
    }
    _ = sql::user::check_password(state, form.clone())
        .await
        .map_err(|_| AuthError::WrongCredentials)?;
    let auth_token = AuthToken {
        username: form.login.to_owned(),
        exp: std::time::SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| AuthError::TokenCreation)?
            .as_secs()
            + TOKEN_DURATION_SECONDS,
    };
    let token = encode(&Header::default(), &auth_token, &KEYS.encoding)
        .map_err(|_| AuthError::TokenCreation)?;

    let jar_res = jar.add(Cookie::new("access_token", token));

    Ok((jar_res, Redirect::to("/index")))
}

#[axum::debug_handler]
pub async fn login_page() -> impl IntoResponse {
    log::debug!("Get /login");
    let template = LoginTemplate {
        page_name: "Login".to_string(),
        action: "/login".to_string(),
        other_page: "/register".to_string(),
        other_page_text: "Or register here".to_string(),
    };
    HtmlTemplate(template)
}

#[axum::debug_handler]
pub async fn register_post(
    State(state): State<Arc<AppState>>,
    jar: CookieJar, // TODO : change this to PrivateCookieJar
    Form(form): Form<user::Model>,
) -> Result<(CookieJar, Redirect), AuthError> {
    _ = sql::user::create_user(state, form).await;
    Ok((jar, Redirect::to("/login")))
}

#[axum::debug_handler]
pub async fn register_page() -> impl IntoResponse {
    log::debug!("Get /login");
    let template = LoginTemplate {
        page_name: "Register".to_string(),
        action: "/register".to_string(),
        other_page: "/login".to_string(),
        other_page_text: "Or login here".to_string(),
    };
    HtmlTemplate(template)
}
