use crate::ApiError;
use crate::AppState;
use crate::ErrorCode;
use crate::ErrorInfo;
use crate::auth::check_auth;
use crate::email::check_email;
use crate::hash::hash_password;
use crate::models::*;
use crate::schema::tokens::dsl::*;
use crate::schema::users::dsl::*;
use axum::Extension;
use axum::extract::State;
use axum::http::StatusCode;
use axum::middleware;
use axum::{Json, Router, routing::get, routing::post};
use diesel::insert_into;
use diesel::prelude::*;
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::{AsyncConnection, RunQueryDsl};
use rand_core::TryRngCore;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use std::time::SystemTime;
use tokio::time::Instant;
use tokio::time::sleep;
use tracing::trace;
use tracing::warn;
use uuid::Uuid;

const NEW_TOKEN_EXPIRY: Duration = Duration::from_hours(24 * 30);
const TOKEN_ROUTE: &str = "/token";
const TOKEN_VALIDATE_ROUTE: &str = constcat::concat!(TOKEN_ROUTE, "/validate");
const TOKEN_USER_ROUTE: &str = constcat::concat!(TOKEN_ROUTE, "/user");

#[derive(Deserialize)]
pub struct SignInRequest {
    email: String,
    password_hash: Vec<u8>,
    allow_renew: bool,
}

#[derive(Serialize)]
pub struct SignInResponse {
    token: Vec<u8>,
    token_expiry: u64,
    renewable: bool,
}

impl From<Token> for SignInResponse {
    fn from(value: Token) -> Self {
        Self {
            token: value.token,
            token_expiry: value
                .expiry
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap() // pretty sure we cant even represent < epoch but should be harmless regardless
                .as_secs(),
            renewable: value.renewable,
        }
    }
}

pub async fn sign_in(
    State(state): State<Arc<AppState>>,
    Json(json): Json<SignInRequest>,
) -> Result<Json<SignInResponse>, ApiError> {
    // since we reject incorrect emails before hashing the password an attacker could use the difference in response time to find valid emails.
    // to avoid this we wait a specified time that should be longer than the time spent hashing to hide the difference
    const TIMING_ATTACK_PROTECTION: Duration = Duration::from_secs(0);
    // starting from where the email is checked and ending once the password is confirmed to be correct
    // there should be no early returns, to avoid any risk of exposing timing information. this means no '?' or .unwrap()

    if !check_email(&json.email) || json.email.len() > 128 {
        return Err(ApiError::WithResponse(
            StatusCode::BAD_REQUEST,
            Json(ErrorInfo {
                error_code: ErrorCode::InvalidRequest,
                error_message: Some(String::from("Invalid email. This shouldnt happen")),
            }),
        ));
    }

    let t1 = Instant::now();
    let mut conn = state.pool.get().await?;
    let state = state.clone();

    conn.transaction(|mut conn| {
        async move {
    // start of 'critial' section (see top of function)
    if let Ok(u) = users
        .select(User::as_select())
        .filter(email.eq(&json.email))
        .first(&mut conn)
        .await
    {
        let password_hash = hash_password(
            state.clone(),
            json.password_hash.try_into().unwrap_or([0; 64]),
            u.salt.try_into().unwrap_or([0; 64]),
        )
        .await;

        if password_hash.unwrap_or_default() == u.password {
            // end of 'critial' section (see top of function)
            // if this code block isnt reached, critical section lasts until the end of the function
            let mut t = vec![0; 64];

            rand_core::OsRng.try_fill_bytes(&mut t)?;

            let token_value: Token = Token {
                user: u.id,
                token: t,
                expiry: SystemTime::now() + NEW_TOKEN_EXPIRY,
                renewable: json.allow_renew,
            };

            insert_into(tokens)
                .values(&token_value)
                .execute(&mut conn)
                .await?;

            return Ok(Json(token_value.into()));
        }
        let elapsed = Instant::now().duration_since(t1);
        trace!(
            "used {:?} out of {:?} hashing",
            elapsed, TIMING_ATTACK_PROTECTION
        );
        if elapsed > TIMING_ATTACK_PROTECTION {
            warn!(
                "took too long to hash password (debug build? overloaded?), timing information may be exposed. took {:?}",
                elapsed
            );
        }
    }
    let elapsed = Instant::now().duration_since(t1);
    sleep(TIMING_ATTACK_PROTECTION.saturating_sub(elapsed)).await;
    Err(ApiError::WithResponse(
        StatusCode::BAD_REQUEST,
        Json(ErrorInfo {
            error_code: ErrorCode::DosentExist,
            error_message: Some(String::from("Invalid email or password.")),
        }),
    ))}
    .scope_boxed()
})
.await
}

pub async fn renew(
    State(state): State<Arc<AppState>>,
    user_id: Extension<Uuid>,
) -> Result<Json<SignInResponse>, ApiError> {
    let mut conn = state.pool.get().await?;

    let mut t = vec![0; 64];
    rand_core::OsRng.try_fill_bytes(&mut t)?;

    let token_value: Token = Token {
        user: user_id.0,
        token: t,
        expiry: SystemTime::now() + NEW_TOKEN_EXPIRY,
        renewable: true,
    };

    insert_into(tokens)
        .values(&token_value)
        .execute(&mut conn)
        .await?;

    Ok(Json(token_value.into()))
}

pub async fn verify() -> StatusCode {
    StatusCode::OK
}

pub async fn get_user(
    State(state): State<Arc<AppState>>,
    user_id: Extension<Uuid>,
) -> Result<Json<PublicUser>, ApiError> {
    let mut conn = state.pool.get().await?;

    if let Ok(u) = users
        .select(PublicUser::as_select())
        .filter(id.eq(user_id.0))
        .get_result(&mut conn)
        .await
    {
        Ok(Json(u))
    } else {
        Err(ApiError::WithResponse(
            StatusCode::NOT_FOUND,
            Json(ErrorInfo {
                error_code: ErrorCode::DosentExist,
                error_message: None,
            }),
        ))
    }
}

pub fn tokens_router(app_state: Arc<AppState>) -> Router {
    let auth_router = Router::new()
        .route(TOKEN_ROUTE, get(renew))
        .route(TOKEN_VALIDATE_ROUTE, get(verify))
        .route(TOKEN_USER_ROUTE, get(get_user))
        .layer(middleware::from_fn_with_state(
            app_state.clone(),
            check_auth,
        ))
        .with_state(app_state.clone());
    Router::new()
        .route(TOKEN_ROUTE, post(sign_in))
        .with_state(app_state)
        .merge(auth_router)
}
