use crate::ApiResponse;
use crate::AppState;
use crate::ErrorCode;
use crate::ErrorInfo;
use crate::auth::check_auth;
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
use diesel_async::RunQueryDsl;
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
    token_expiry: Option<u64>,
    renewable: bool,
}

impl From<Token> for SignInResponse {
    fn from(value: Token) -> Self {
        Self {
            token: value.token,
            token_expiry: value
                .expiry
                .and_then(|x| x.duration_since(SystemTime::now()).ok())
                .map(|x| x.as_secs()),
            renewable: value.renewable,
        }
    }
}

pub async fn sign_in(
    state: State<Arc<AppState>>,
    Json(json): Json<SignInRequest>,
) -> ApiResponse<SignInResponse> {
    // since we reject incorrect emails before hashing the password
    // an attacker could use the difference in response time to find valid emails.
    // to avoid this we wait a specified time
    // that should be longer than the time spent hashing to hide the difference
    const TIMING_ATTACK_PROTECTION: Duration = Duration::from_secs(1);

    let t1 = Instant::now();
    let conn = state.pool.get().await;
    if conn.is_err() {
        warn!("failed to aquire db connection");
        return ApiResponse::BadNoInfo(StatusCode::INTERNAL_SERVER_ERROR);
    }
    let mut conn = conn.unwrap();

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
            let mut t = vec![0; 64];

            if rand_core::OsRng.try_fill_bytes(&mut t).is_err() {
                warn!("failed to get rng bytes from OsRng");
                return ApiResponse::BadNoInfo(StatusCode::SERVICE_UNAVAILABLE);
            }

            let token_value: Token = Token {
                user: u.id,
                token: t,
                expiry: SystemTime::now().checked_add(NEW_TOKEN_EXPIRY),
                renewable: json.allow_renew,
            };

            if insert_into(tokens)
                .values(&token_value)
                .execute(&mut conn)
                .await
                .is_err()
            {
                trace!("failed to insert new token");
                return ApiResponse::BadNoInfo(StatusCode::INTERNAL_SERVER_ERROR);
            }

            return ApiResponse::Good(StatusCode::OK, Json(token_value.into()));
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
    return ApiResponse::Bad(
        StatusCode::BAD_REQUEST,
        Json(ErrorInfo {
            error_code: ErrorCode::UserDosentExist,
            error_message: Some(String::from("Invalid email or password.")),
        }),
    );
}

pub async fn renew(
    State(state): State<Arc<AppState>>,
    user_id: Extension<Uuid>,
) -> ApiResponse<SignInResponse> {
    let conn = state.pool.get().await;
    if conn.is_err() {
        warn!("failed to aquire db connection");
        return ApiResponse::BadNoInfo(StatusCode::INTERNAL_SERVER_ERROR);
    }
    let mut conn = conn.unwrap();

    let mut t = vec![0; 64];
    if rand_core::OsRng.try_fill_bytes(&mut t).is_err() {
        warn!("failed to get rng bytes from OsRng");
        return ApiResponse::BadNoInfo(StatusCode::SERVICE_UNAVAILABLE);
    }

    let token_value: Token = Token {
        user: user_id.0,
        token: t,
        expiry: SystemTime::now().checked_add(NEW_TOKEN_EXPIRY),
        renewable: true,
    };

    insert_into(tokens)
        .values(&token_value)
        .execute(&mut conn)
        .await
        .unwrap();

    return ApiResponse::Good(StatusCode::OK, Json(token_value.into()));
}

pub async fn verify() -> StatusCode {
    StatusCode::OK
}

pub async fn get_user(
    State(state): State<Arc<AppState>>,
    user_id: Extension<Uuid>,
) -> ApiResponse<PublicUser> {
    let conn = state.pool.get().await;
    if conn.is_err() {
        warn!("failed to aquire db connection");
        return ApiResponse::BadNoInfo(StatusCode::INTERNAL_SERVER_ERROR);
    }
    let mut conn = conn.unwrap();

    if let Ok(u) = users
        .select(PublicUser::as_select())
        .filter(id.eq(user_id.0))
        .get_result(&mut conn)
        .await
    {
        return ApiResponse::Good(StatusCode::OK, Json(u));
    } else {
        return ApiResponse::Bad(
            StatusCode::BAD_REQUEST,
            Json(ErrorInfo {
                error_code: ErrorCode::UserDosentExist,
                error_message: None,
            }),
        );
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
