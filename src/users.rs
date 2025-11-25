use crate::AppState;
use crate::auth;
use crate::models::*;
use crate::schema::users::dsl::*;
use crate::users;
use argon2::PasswordHash;
use axum::extract::Path;
use axum::extract::State;
use axum::middleware;
use axum::{Json, Router, routing::get, routing::post};
use diesel::insert_into;
use diesel::prelude::*;
use password_hash::SaltString;
use password_hash::rand_core;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use std::time::SystemTime;
use tokio::time::Instant;
use tokio::time::sleep;
use tracing::trace;
use tracing::warn;
use uuid::Uuid;

const ROUTE_ORIGIN: &str = "/api/v0";
const USERS_ROUTE: &str = constcat::concat!(ROUTE_ORIGIN, "/users");
const USER_ID_ROUTE: &str = constcat::concat!(USERS_ROUTE, "/{usr_id}");

#[derive(Deserialize)]
struct SignUp {
    pub username: String,
    pub password: String,
    pub email: String,
}

#[derive(Serialize)]
struct SignUpResult {
    pub was_created: bool,
    pub error_message: String,
}

#[derive(Deserialize)]
struct SignIn {
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
struct SignInResult {
    pub success: bool,
    pub login_token: Vec<u8>,
    pub token_expiry: SystemTime,
    pub can_renew: bool,
    pub error_message: String,
}

#[derive(Deserialize)]
struct Search {
    pub search_term: String,
}

#[derive(Serialize)]
struct SearchResult {
    pub result_count: i64,
    pub results: Vec<PublicUser>,
}

async fn get_user(
    State(state): State<Arc<AppState>>,
    Path(usr_id): Path<Uuid>,
) -> Json<PublicUser> {
    let mut db_connection = state.pool.get().unwrap();
    if let Ok(usr) = users
        .filter(id.eq(usr_id))
        .select(User::as_select())
        .get_result(&mut db_connection)
    {
        return Json(PublicUser {
            id: usr.id,
            username: usr.username,
        });
    }
    Json(PublicUser {
        id: usr_id,
        username: String::new(),
    })
}
async fn find_user(
    State(state): State<Arc<AppState>>,
    Json(json): Json<Search>,
) -> Json<SearchResult> {
    let mut db_connection = state.pool.get().unwrap();
    let users_list = users
        .select(PublicUser::as_select())
        .filter(username.like("%".to_owned() + &json.search_term + "%"))
        .load(&mut db_connection)
        .unwrap();
    trace!("got {:#?}", users_list);
    Json(SearchResult {
        result_count: users_list.len() as i64,
        results: users_list,
    })
}

async fn sign_up(
    State(state): State<Arc<AppState>>,
    Json(json): Json<SignUp>,
) -> Json<SignUpResult> {
    let mut db_connection = state.pool.get().unwrap();
    if users
        .count()
        .filter(username.eq(&json.username))
        .or_filter(email.eq(&json.email))
        .get_result(&mut db_connection)
        .unwrap_or(1)
        != 0
    {
        return Json(SignUpResult {
            was_created: false,
            error_message: "Username or email already in use.".to_string(),
        });
    }
    let password_salt: SaltString = SaltString::generate(rand_core::OsRng);
    let hash = PasswordHash::generate(
        state.password_hasher.clone(),
        json.password,
        password_salt.as_salt(),
    )
    .unwrap();
    let password_hash = hash.hash.unwrap();
    let new_user: NewUser = NewUser {
        username: &json.username,
        password: password_hash.as_bytes(),
        salt: password_salt.as_str().as_bytes(),
        email: &json.email,
    };

    insert_into(users)
        .values(new_user)
        .execute(&mut db_connection)
        .unwrap();

    Json(SignUpResult {
        was_created: true,
        error_message: String::new(),
    })
}
async fn sign_in(
    State(state): State<Arc<AppState>>,
    Json(json): Json<SignIn>,
) -> Json<SignInResult> {
    let mut db_connection = state.pool.get().unwrap();
    if let Ok(user) = users
        .select(User::as_select())
        .filter(email.eq(&json.email))
        .get_result(&mut db_connection)
    {
        trace!("found matching email");
        let start: Instant = Instant::now();
        let password_salt: SaltString =
            SaltString::from_b64(&String::from_utf8_lossy(&user.salt)).unwrap();
        let hash = PasswordHash::generate(
            state.password_hasher.clone(),
            json.password,
            password_salt.as_salt(),
        )
        .unwrap();
        if hash.hash.unwrap().as_bytes() == user.password {
            let end: Instant = Instant::now();
            let max_wait_time: Duration = Duration::from_millis(2500);
            let wait_time: Duration = max_wait_time.saturating_sub(end - start);
            if wait_time < Duration::from_millis(25) {
                warn!(
                    "took {:#?} to compute hash but we are only protected from timing up to {:#?}",
                    end - start,
                    max_wait_time
                );
            } else {
                sleep(wait_time).await;
            }
            return Json(SignInResult {
                success: true,
                login_token: vec![
                    1, 2, 3, 4, 5, 6, 7, 8, 1, 2, 3, 4, 5, 6, 7, 8, 1, 2, 3, 4, 5, 6, 7, 8, 1, 2,
                    3, 4, 5, 6, 7, 8,
                ],
                token_expiry: SystemTime::now() + Duration::from_secs(60 * 60 * 24 * 30), // valid for 30 days
                can_renew: true,
                error_message: String::new(),
            });
        }
    }
    sleep(Duration::from_millis(2500)).await;
    Json(SignInResult {
        success: false,
        login_token: vec![],
        token_expiry: SystemTime::now(),
        can_renew: false,
        error_message: "invalid email or password".to_string(),
    })
}
pub fn users_router(app_state: Arc<AppState>) -> Router {
    Router::new()
        .route(
            USERS_ROUTE,
            get(find_user)
                .layer(middleware::from_fn(auth::check_auth))
                .post(sign_up),
        )
        .route(constcat::concat!(USERS_ROUTE, "/sign_in"), post(sign_in))
        .route(USER_ID_ROUTE, get(get_user))
        .with_state(app_state)
}
