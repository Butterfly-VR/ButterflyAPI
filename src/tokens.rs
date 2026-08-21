use crate::ApiError;
use crate::AppState;
use crate::ErrorCode;
use crate::ErrorInfo;
use crate::auth::check_auth;
use crate::email::check_email;
use crate::hash::hash_password;
use crate::models::IpInfo;
use crate::models::{PublicUserInfo, Token, User};
use crate::schema::ip_infos;
use crate::schema::tokens::dsl::tokens;
use crate::schema::users::dsl::{email, id, users};
use axum::Extension;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::http::StatusCode;
use axum::middleware;
use axum::{Json, Router, routing::get, routing::post};
use diesel::insert_into;
use diesel::prelude::*;
use diesel::update;
use diesel_async::{AsyncConnection, RunQueryDsl};
use ipnet::IpNet;
use rand::TryRng;
use rand::rngs::SysRng;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::str::FromStr;
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
    token_expires: u64,
    renewable: bool,
}

// we cant represent a time earlier than the epoch so this never panics
#[allow(clippy::fallible_impl_from)]
impl From<Token> for SignInResponse {
    fn from(value: Token) -> Self {
        Self {
            token: value.token,
            token_expires: value
                .expires
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            renewable: value.renewable,
        }
    }
}

pub async fn sign_in(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(json): Json<SignInRequest>,
) -> Result<Json<SignInResponse>, ApiError> {
    // since we reject incorrect emails before hashing the password an attacker
    // could use the difference in response time to find valid emails.
    // to avoid this we wait a specified time that should be longer
    // than the time spent hashing to hide the difference
    // starting from where the email is checked and ending once the password is confirmed to be correct
    // there should be no early returns, to avoid any risk of exposing timing information.
    // this means no '?' or .unwrap()
    const TIMING_ATTACK_PROTECTION: Duration = Duration::from_millis(500);
    const LOGIN_ATTEMPT_LIMIT: i16 = 10;
    const LOGIN_ATTEMPT_LIMIT_RESET_DURATION: Duration = Duration::from_hours(1);

    if json.email.len() > 128 || !check_email(&json.email) {
        return Err(ApiError::WithResponse(
            StatusCode::BAD_REQUEST,
            Json(ErrorInfo {
                error_code: ErrorCode::InvalidRequest,
                error_message: Some(String::from("Invalid email")),
            }),
        ));
    }

    let t1 = Instant::now();
    let mut conn = state.pool.get().await?;
    let state = state.clone();

    conn.transaction(async |mut conn| {
        let ip = headers
            .get("x-forwarded-for")
            .ok_or(ApiError::WithCode(StatusCode::INTERNAL_SERVER_ERROR))?;
        let ip = IpNet::new(IpAddr::from_str(ip.to_str()?)?, 32)?;

        if let Some(ip_info) = ip_infos::table.select((ip_infos::login_attempts, ip_infos::login_attempts_reset)).filter(ip_infos::ip.eq(ip))
            .first::<(i16, SystemTime)>(&mut conn)
            .await
            .optional()?
        {
            if ip_info.1 < SystemTime::now() {
                update(ip_infos::table.filter(ip_infos::ip.eq(ip)))
                    .set((
                        ip_infos::login_attempts.eq(0),
                        ip_infos::login_attempts_reset
                            .eq(SystemTime::now() + LOGIN_ATTEMPT_LIMIT_RESET_DURATION),
                    ))
                    .execute(&mut conn)
                    .await?;
            } else if ip_info.0 >= LOGIN_ATTEMPT_LIMIT {
                return Err(ApiError::WithCode(StatusCode::TOO_MANY_REQUESTS));
            }
        } else {
            let entry = IpInfo {
                ip,
                accounts_created: 0,
                account_creation_count_reset: SystemTime::now(),
                login_attempts: 0,
                login_attempts_reset: SystemTime::now() + LOGIN_ATTEMPT_LIMIT_RESET_DURATION,
            };
            insert_into(ip_infos::table)
                .values(entry)
                .execute(&mut conn)
                .await?;
        }

        // start of 'critical' section (see top of function)
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
                // end of 'critical' section (see top of function)
                // if this code block isnt reached, critical section lasts until the end of the function
                let mut t = vec![0; 64];

                SysRng.try_fill_bytes(&mut t)?;

                let token_value: Token = Token {
                    user: u.id,
                    token: t,
                    renewable: json.allow_renew,
                    expires: SystemTime::now() + NEW_TOKEN_EXPIRY,
                };

                insert_into(tokens)
                    .values(&token_value)
                    .execute(&mut conn)
                    .await?;

                return Ok(Ok(Json(token_value.into())));
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

        update(ip_infos::table.filter(ip_infos::ip.eq(ip)))
            .set(ip_infos::login_attempts.eq(ip_infos::login_attempts + 1))
            .execute(&mut conn)
            .await?;

        let elapsed = Instant::now().duration_since(t1);
        sleep(TIMING_ATTACK_PROTECTION.saturating_sub(elapsed)).await;
        Ok(Err(ApiError::WithResponse(
            StatusCode::BAD_REQUEST,
            Json(ErrorInfo {
                error_code: ErrorCode::DosentExist,
                error_message: Some(String::from("Invalid email or password.")),
            }),
        )))
    })
    .await?
}

pub async fn renew(
    State(state): State<Arc<AppState>>,
    user_id: Extension<Uuid>,
) -> Result<Json<SignInResponse>, ApiError> {
    let mut conn = state.pool.get().await?;

    let mut t = vec![0; 64];
    SysRng.try_fill_bytes(&mut t)?;

    let token_value: Token = Token {
        user: user_id.0,
        token: t,
        renewable: true,
        expires: SystemTime::now() + NEW_TOKEN_EXPIRY,
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
) -> Result<Json<PublicUserInfo>, ApiError> {
    let mut conn = state.pool.get().await?;

    users
        .select(PublicUserInfo::as_select())
        .filter(id.eq(user_id.0))
        .first(&mut conn)
        .await
        .map(Json)
        .map_err(|_| {
            ApiError::WithResponse(
                StatusCode::NOT_FOUND,
                Json(ErrorInfo {
                    error_code: ErrorCode::DosentExist,
                    error_message: None,
                }),
            )
        })
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
