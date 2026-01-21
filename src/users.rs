use crate::ApiError;
use crate::AppState;
use crate::ErrorCode;
use crate::ErrorInfo;
use crate::auth;
use crate::email::EmailType;
use crate::email::check_email;
use crate::email::send_email;
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
use axum::response::IntoResponse;
use axum::{Json, Router, routing::get, routing::post};
use diesel::insert_into;
use diesel::prelude::*;
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::{AsyncConnection, RunQueryDsl};
use rand_core::TryRngCore;
use serde::Deserialize;
use std::sync::Arc;
use std::time::Duration;
use std::time::SystemTime;
use tracing::info;
use uuid::Uuid;

const USERS_ROUTE: &str = "/user";
const USER_ID_ROUTE: &str = constcat::concat!(USERS_ROUTE, "/{usr_id}");
const USER_EMAIL_VERIFY_ROUTE: &str = constcat::concat!(USER_ID_ROUTE, "/verify/{token}");

#[derive(Deserialize)]
pub struct SignUpRequest {
    pub username: String,
    pub password_hash: Vec<u8>,
    pub email: String,
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
                let mut token = [0; 64];
                rand_core::OsRng.try_fill_bytes(&mut token)?;

                let id = Uuid::new_v4();

                send_email(
                    &json.email,
                    json.username.clone(),
                    EmailType::EmailVerify(token, id),
                )
                .await?;

                // delete any previous sign up attempts
                diesel::delete(unverified_users::table)
                    .filter(unverified_users::username.eq(&json.username))
                    .or_filter(unverified_users::email.eq(&json.email))
                    .execute(&mut conn)
                    .await?;

                let new_user: UnverifiedUser = UnverifiedUser {
                    id,
                    username: json.username,
                    password: password_hash,
                    salt: Vec::from(password_salt),
                    email: json.email,
                    token: Vec::from(token),
                    expiry: SystemTime::now() + Duration::from_mins(15),
                };

                insert_into(unverified_users::table)
                    .values::<UnverifiedUser>(new_user)
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

pub enum GetUserResult {
    PublicUser(Json<PublicUserInfo>),
    User(Json<User>),
}

impl IntoResponse for GetUserResult {
    fn into_response(self) -> axum::response::Response {
        match self {
            GetUserResult::PublicUser(user) => user.into_response(),
            GetUserResult::User(user) => user.into_response(),
        }
    }
}

pub async fn get_user(
    State(state): State<Arc<AppState>>,
    Path(usr_id): Path<Uuid>,
    Extension(user_id): Extension<Uuid>,
) -> Result<GetUserResult, ApiError> {
    let mut conn = state.pool.get().await?;

    if let Ok(Some(mut user)) = users::table
        .select(User::as_select())
        .filter(users::id.eq(usr_id))
        .first(&mut conn)
        .await
        .optional()
    {
        if user_id == user.id {
            // todo: this is kinda jank
            user.homeworld = Some(user.homeworld.unwrap_or(Uuid::nil()));
            user.avatar = Some(user.avatar.unwrap_or(Uuid::from_u64_pair(0, 1))); // yeah thats not hacky at all
            return Ok(GetUserResult::User(Json(user)));
        } else {
            return Ok(GetUserResult::PublicUser(Json(user.into())));
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

pub async fn verify_email(
    State(state): State<Arc<AppState>>,
    Path((usr_id, token)): Path<(Uuid, String)>,
) -> Result<(), ApiError> {
    let Ok(token) = hex::decode(token) else {
        return Err(ApiError::WithResponse(
            StatusCode::BAD_REQUEST,
            Json(ErrorInfo {
                error_code: ErrorCode::InvalidRequest,
                error_message: Some("invalid token supplied".to_owned()),
            }),
        ));
    };

    let mut conn = state.pool.get().await?;

    conn.transaction(|mut conn| {
        async move {
            if let Some(user) = unverified_users::table
                .select(UnverifiedUser::as_select())
                .filter(unverified_users::id.eq(usr_id))
                .get_result(&mut conn)
                .await
                .optional()?
            {
                if user.token == token && user.expiry > SystemTime::now() {
                    let new_user: User = User {
                        id: user.id,
                        username: user.username,
                        password: user.password,
                        salt: user.salt,
                        email: user.email,
                        permisions: Vec::new(),
                        trust: 0,
                        homeworld: None,
                        avatar: None,
                    };
                    insert_into(users::table)
                        .values(new_user)
                        .execute(&mut conn)
                        .await?;
                    diesel::delete(unverified_users::table)
                        .filter(unverified_users::id.eq(usr_id))
                        .execute(&mut conn)
                        .await?;
                    Ok(())
                } else {
                    Err(ApiError::WithResponse(
                        StatusCode::BAD_REQUEST,
                        Json(ErrorInfo {
                            error_code: ErrorCode::InvalidRequest,
                            error_message: Some(
                                "Token was expired or invalid. Try signing up again.".to_owned(),
                            ),
                        }),
                    ))
                }
            } else if users::table
                .count()
                .filter(users::id.eq(usr_id))
                .get_result::<i64>(&mut conn)
                .await?
                != 0
            {
                Err(ApiError::WithResponse(
                    StatusCode::BAD_REQUEST,
                    Json(ErrorInfo {
                        error_code: ErrorCode::InvalidRequest,
                        error_message: Some("User is already verified".to_owned()),
                    }),
                ))
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
        .scope_boxed()
    })
    .await
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
        .route(USER_EMAIL_VERIFY_ROUTE, get(verify_email))
        .with_state(app_state)
}
