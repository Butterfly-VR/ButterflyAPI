use crate::ApiError;
use crate::AppState;
use crate::ErrorCode;
use crate::ErrorInfo;
use crate::auth;
use crate::email::EmailType;
use crate::email::check_email;
use crate::email::send_email;
use crate::hash::hash_password;
use crate::models::IpInfo;
use crate::models::{PublicUserInfo, UnverifiedUser, User};
use crate::schema::ip_infos;
use crate::schema::unverified_users;
use crate::schema::users;
use axum::Extension;
use axum::extract::Path;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::http::StatusCode;
use axum::middleware;
use axum::response::IntoResponse;
use axum::{Json, Router, routing::get, routing::post};
use diesel::insert_into;
use diesel::prelude::*;
use diesel::update;
use diesel_async::{AsyncConnection, RunQueryDsl};
use ipnet::IpNet;
use rand::TryRng;
use rand::rngs::SysRng;
use serde::Deserialize;
use std::net::IpAddr;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use std::time::SystemTime;
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
    headers: HeaderMap,
    Json(json): Json<SignUpRequest>,
) -> Result<(), ApiError> {
    const ACCOUNT_CREATION_LIMIT: i16 = 15;
    // every year
    const ACCOUNT_CREATION_LIMIT_RESET_DURATION: Duration = Duration::from_hours(24 * 365);

    let mut conn = state.pool.get().await?;
    let state = state.clone();

    if json.username.len() < 3 || json.username.len() > 32 || json.email.len() > 128 {
        return Err(ApiError::WithResponse(
            StatusCode::BAD_REQUEST,
            Json(ErrorInfo {
                error_code: ErrorCode::BadRequestLength,
                error_message: Some(String::from("Username or email was wrong length.")),
            }),
        ));
    }

    if !check_email(&json.email) {
        return Err(ApiError::WithResponse(
            StatusCode::BAD_REQUEST,
            Json(ErrorInfo {
                error_code: ErrorCode::InvalidRequest,
                error_message: Some(String::from("Invalid email.")),
            }),
        ));
    }

    conn.transaction(async |mut conn| {
        {
            let ip = headers
                .get("x-forwarded-for")
                .ok_or(ApiError::WithCode(StatusCode::INTERNAL_SERVER_ERROR))?;
            let ip = IpNet::new(IpAddr::from_str(ip.to_str()?)?, 32)?;

            if let Some(ip_info) = update(ip_infos::table.filter(ip_infos::ip.eq(ip)))
                .set(ip_infos::accounts_created.eq(ip_infos::accounts_created + 1))
                .returning((
                    ip_infos::accounts_created,
                    ip_infos::account_creation_count_reset,
                ))
                .get_result::<(i16, SystemTime)>(&mut conn)
                .await
                .optional()?
            {
                if ip_info.1 < SystemTime::now() {
                    update(ip_infos::table.filter(ip_infos::ip.eq(ip)))
                        .set((
                            ip_infos::accounts_created.eq(0),
                            ip_infos::account_creation_count_reset
                                .eq(SystemTime::now() + ACCOUNT_CREATION_LIMIT_RESET_DURATION),
                        ))
                        .execute(&mut conn)
                        .await?;
                } else if ip_info.0 >= ACCOUNT_CREATION_LIMIT {
                    return Ok::<Result<(), ApiError>, ApiError>(Err(ApiError::WithCode(
                        StatusCode::TOO_MANY_REQUESTS,
                    )));
                }
            } else {
                let entry = IpInfo {
                    ip,
                    accounts_created: 1,
                    account_creation_count_reset: SystemTime::now()
                        + ACCOUNT_CREATION_LIMIT_RESET_DURATION,
                    login_attempts: 0,
                    login_attempts_reset: SystemTime::now(),
                };
                insert_into(ip_infos::table)
                    .values(entry)
                    .execute(&mut conn)
                    .await?;
            }

            if users::table
                .count()
                .filter(users::username.eq(&json.username))
                .or_filter(users::email.eq(&json.email))
                .first::<i64>(&mut conn)
                .await?
                != 0
            {
                return Ok(Err(ApiError::WithResponse(
                    StatusCode::BAD_REQUEST,
                    Json(ErrorInfo {
                        error_code: ErrorCode::AlreadyExists,
                        error_message: Some(String::from("Username or email already in use.")),
                    }),
                )));
            }

            let mut password_salt = [0; 64];
            SysRng.try_fill_bytes(&mut password_salt)?;

            let password_hash = hash_password(
                state.clone(),
                json.password_hash.try_into().unwrap_or([0; 64]),
                password_salt,
            )
            .await
            .map_err(|_| ApiError::WithCode(StatusCode::INTERNAL_SERVER_ERROR))?;

            let mut token = [0; 64];
            SysRng.try_fill_bytes(&mut token)?;

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
                expires: SystemTime::now() + Duration::from_mins(15),
            };

            insert_into(unverified_users::table)
                .values::<UnverifiedUser>(new_user)
                .execute(&mut conn)
                .await?;
            Ok(Ok(()))
        }
    })
    .await?
}

pub enum GetUserResult {
    PublicUser(Json<PublicUserInfo>),
    User(Json<User>),
}

impl IntoResponse for GetUserResult {
    fn into_response(self) -> axum::response::Response {
        match self {
            Self::PublicUser(user) => user.into_response(),
            Self::User(user) => user.into_response(),
        }
    }
}

pub async fn get_user(
    State(state): State<Arc<AppState>>,
    Path(target_user): Path<Uuid>,
    Extension(requesting_user): Extension<Uuid>,
) -> Result<GetUserResult, ApiError> {
    let mut conn = state.pool.get().await?;

    if let Ok(Some(mut user)) = users::table
        .select(User::as_select())
        .filter(users::id.eq(target_user))
        .first(&mut conn)
        .await
        .optional()
    {
        if requesting_user == user.id {
            // homeworld and avatar need to be nullable to avoid circular dependacies
            // but we want to always return something valid
            // these values should corrospond to objects uploaded by the ButterflyDev account
            user.homeworld = Some(user.homeworld.unwrap_or(Uuid::nil()));
            user.avatar = Some(user.avatar.unwrap_or(Uuid::from_u64_pair(0, 1)));
            return Ok(GetUserResult::User(Json(user)));
        }
        return Ok(GetUserResult::PublicUser(Json(user.into())));
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
) -> Result<&'static str, ApiError> {
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

    conn.transaction(async |mut conn| {
        if let Some(user) = unverified_users::table
            .select(UnverifiedUser::as_select())
            .filter(unverified_users::id.eq(usr_id))
            .first(&mut conn)
            .await
            .optional()?
        {
            if user.token == token && user.expires > SystemTime::now() {
                let new_user: User = User {
                    id: user.id,
                    username: user.username,
                    password: user.password,
                    salt: user.salt,
                    email: user.email,
                    permissions_level: 0,
                    trust: 0,
                    homeworld: None,
                    avatar: None,
                    instance: None,
                    identifier: None,
                    delete_at: None,
                    can_login: true,
                    upload_quota_used: 0,
                    download_quota_used: 0,
                };
                insert_into(users::table)
                    .values(new_user)
                    .execute(&mut conn)
                    .await?;
                diesel::delete(unverified_users::table)
                    .filter(unverified_users::id.eq(usr_id))
                    .execute(&mut conn)
                    .await?;
                Ok("Your email has been verified. You may now close this page.")
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
            .first::<i64>(&mut conn)
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
