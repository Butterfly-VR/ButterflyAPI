use crate::ApiResponse;
use crate::AppState;
use crate::ErrorCode;
use crate::ErrorInfo;
use crate::auth;
use crate::hash::hash_password;
use crate::models::*;
use crate::schema::users::dsl::*;
use axum::extract::Path;
use axum::extract::State;
use axum::http;
use axum::http::StatusCode;
use axum::middleware;
use axum::{Json, Router, routing::get, routing::post};
use diesel::insert_into;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use rand_core::TryRngCore;
use serde::Deserialize;
use std::sync::Arc;
use tracing::info;
use tracing::trace;
use tracing::warn;
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

impl<'a> From<NewUser<'a>> for User {
    fn from(value: NewUser) -> Self {
        Self {
            id: Uuid::new_v4(),
            username: value.username.to_owned(),
            password: value.password.to_owned(),
            salt: value.salt.to_owned(),
            email: value.email.to_owned(),
            verified_email: false,
            permisions: crate::models::PermissionLevel::None as i16,
            trust: 0,
            homeworld: None,
            avatar: None,
        }
    }
}

pub async fn sign_up(
    state: State<Arc<AppState>>,
    Json(json): Json<SignUpRequest>,
) -> ApiResponse<()> {
    trace!("handler start");
    let conn = state.pool.get().await;
    if conn.is_err() {
        warn!("failed to aquire db connection");
        return ApiResponse::BadNoInfo(StatusCode::INTERNAL_SERVER_ERROR);
    }
    let mut conn = conn.unwrap();

    trace!("count request");
    if users
        .count()
        .filter(username.eq(&json.username))
        .or_filter(email.eq(&json.email))
        .get_result(&mut conn)
        .await
        .unwrap_or(1)
        != 0
    {
        trace!("got bad request: duplicate user");
        return ApiResponse::Bad(
            http::StatusCode::BAD_REQUEST,
            Json(ErrorInfo {
                code: ErrorCode::UserExists,
                message: Some(String::from("Username or email already in use.")),
            }),
        );
    }

    let mut password_salt = [0; 64];
    if rand_core::OsRng.try_fill_bytes(&mut password_salt).is_err() {
        warn!("failed to get rng bytes from OsRng");
        return ApiResponse::BadNoInfo(StatusCode::SERVICE_UNAVAILABLE);
    }

    trace!("hash start");
    if let Some(password_hash) = hash_password(
        state.clone(),
        json.password_hash.try_into().unwrap_or([0; 64]),
        password_salt.clone(),
    )
    .await
    .ok()
    {
        let new_user: NewUser = NewUser {
            username: &json.username,
            password: &password_hash,
            salt: &password_salt,
            email: &json.email,
        };
        let result = insert_into(users)
            .values::<User>(new_user.into())
            .execute(&mut conn)
            .await;
        if result.is_ok() {
            return ApiResponse::Good(http::StatusCode::OK, Json(()));
        } else {
            trace!("failed to insert new user: {:?}", result.unwrap_err());
            return ApiResponse::BadNoInfo(StatusCode::INTERNAL_SERVER_ERROR);
        }
    } else {
        // most likely cause is that we didnt have a block available
        info!("failed to hash password, probably because no available memory blocks");
        return ApiResponse::BadNoInfo(http::StatusCode::SERVICE_UNAVAILABLE);
    }
}

pub async fn get_user(
    State(state): State<Arc<AppState>>,
    Path(usr_id): Path<Uuid>,
) -> ApiResponse<PublicUser> {
    let conn = state.pool.get().await;
    if conn.is_err() {
        warn!("failed to aquire db connection");
        return ApiResponse::BadNoInfo(StatusCode::SERVICE_UNAVAILABLE);
    }
    let mut conn = conn.unwrap();

    if let Ok(Some(user)) = users
        .select(PublicUser::as_select())
        .filter(id.eq(usr_id))
        .first(&mut conn)
        .await
        .optional()
    {
        return ApiResponse::Good(StatusCode::OK, Json(user));
    }
    trace!("request for non existant user uuid");
    ApiResponse::BadNoInfo(StatusCode::NOT_FOUND)
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
