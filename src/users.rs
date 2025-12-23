use crate::ApiError;
use crate::AppState;
use crate::ErrorCode;
use crate::ErrorInfo;
use crate::auth;
use crate::email::check_email;
use crate::hash::hash_password;
use crate::models::*;
use crate::schema::unverified_users;
use crate::schema::users;
use axum::Extension;
use axum::extract::Path;
use axum::extract::State;
use axum::http;
use axum::http::StatusCode;
use axum::middleware;
use axum::{Json, Router, routing::get, routing::post};
use diesel::insert_into;
use diesel::prelude::*;
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::{AsyncConnection, RunQueryDsl};
use rand_core::TryRngCore;
use serde::Deserialize;
use serde::Serialize;
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

const USERS_ROUTE: &str = "/user";
const USER_ID_ROUTE: &str = constcat::concat!(USERS_ROUTE, "/{usr_id}");

#[derive(Deserialize)]
pub struct SignUpRequest {
    pub username: String,
    pub password_hash: Vec<u8>,
    pub email: String,
}

pub struct NewUser<'a> {
    pub username: &'a str,
    pub password: &'a [u8],
    pub salt: &'a [u8],
    pub email: &'a str,
}

impl<'a> From<NewUser<'a>> for UnverifiedUser {
    fn from(value: NewUser) -> Self {
        Self {
            id: Uuid::new_v4(),
            username: value.username.to_owned(),
            password: value.password.to_owned(),
            salt: value.salt.to_owned(),
            email: value.email.to_owned(),
        }
    }
}

pub async fn sign_up(
    State(state): State<Arc<AppState>>,
    Json(json): Json<SignUpRequest>,
) -> Result<(), ApiError> {
    let mut conn = state.pool.get().await?;
    let state = state.clone();

    if json.username.len() < 6 || json.username.len() > 32 || json.email.len() > 128 {
        return Err(ApiError::WithResponse(
            StatusCode::BAD_REQUEST,
            Json(ErrorInfo {
                error_code: ErrorCode::BadRequestLength,
                error_message: Some(String::from(
                    "Username or email was wrong length. This shouldnt happen",
                )),
            }),
        ));
    }

    if !check_email(&json.email) {
        return Err(ApiError::WithResponse(
            StatusCode::BAD_REQUEST,
            Json(ErrorInfo {
                error_code: ErrorCode::InvalidRequest,
                error_message: Some(String::from("Invalid email. This shouldnt happen")),
            }),
        ));
    }

    conn.transaction(|mut conn| {
        async move {
            if users::table
                .count()
                .filter(users::username.eq(&json.username))
                .or_filter(users::email.eq(&json.email))
                .get_result::<i64>(&mut conn)
                .await?
                != 0
            {
                return Err(ApiError::WithResponse(
                    http::StatusCode::BAD_REQUEST,
                    Json(ErrorInfo {
                        error_code: ErrorCode::AlreadyExists,
                        error_message: Some(String::from("Username or email already in use.")),
                    }),
                ));
            }

            let mut password_salt = [0; 64];
            rand_core::OsRng.try_fill_bytes(&mut password_salt)?;

            if let Ok(password_hash) = hash_password(
                state.clone(),
                json.password_hash.try_into().unwrap_or([0; 64]),
                password_salt,
            )
            .await
            {
                let new_user: NewUser = NewUser {
                    username: &json.username,
                    password: &password_hash,
                    salt: &password_salt,
                    email: &json.email,
                };
                insert_into(unverified_users::table)
                    .values::<UnverifiedUser>(new_user.into())
                    .execute(&mut conn)
                    .await?;
                Ok(())
            } else {
                // most likely cause is that we didnt have a block available
                info!(
                    "failed to hash password, probably because we had no available memory blocks"
                );
                Err(ApiError::WithCode(http::StatusCode::SERVICE_UNAVAILABLE))
            }
        }
        .scope_boxed()
    })
    .await
}

#[derive(Serialize)]
pub enum GetUserResult {
    PublicUser(PublicUser),
    #[allow(dead_code)]
    FriendUser(()), // todo after friends implemented
    User(User),
}

pub async fn get_user(
    State(state): State<Arc<AppState>>,
    Path(usr_id): Path<Uuid>,
    Extension(user_id): Extension<Uuid>,
) -> Result<Json<GetUserResult>, ApiError> {
    let mut conn = state.pool.get().await?;

    if let Ok(Some(user)) = users::table
        .select(User::as_select())
        .filter(users::id.eq(usr_id))
        .first(&mut conn)
        .await
        .optional()
    {
        if user_id == user.id {
            return Ok(Json(GetUserResult::User(user)));
        } else {
            return Ok(Json(GetUserResult::PublicUser(user.into())));
        }
    }
    Err(ApiError::WithResponse(
        StatusCode::NOT_FOUND,
        Json(ErrorInfo {
            error_code: ErrorCode::DosentExist,
            error_message: None,
        }),
    ))
}

pub fn users_router(app_state: Arc<AppState>) -> Router {
    Router::new()
        .route(USERS_ROUTE, post(sign_up))
        .route(
            USER_ID_ROUTE,
            get(get_user).layer(middleware::from_fn_with_state(
                app_state.clone(),
                auth::check_auth,
            )),
        )
        .with_state(app_state)
}
