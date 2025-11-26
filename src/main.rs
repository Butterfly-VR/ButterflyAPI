use axum::Json;
use axum::response::IntoResponse;
use axum::{Router, http, middleware, routing::get};
use diesel::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use dotenvy::dotenv;

use serde::Serialize;
use std::collections::{HashMap, VecDeque};
use std::net::IpAddr;
use std::time::SystemTime;
use std::{env, sync::Arc};
use tokio::sync::{Mutex, RwLock};
use tokio::task::yield_now;
use tower_http::trace::TraceLayer;
mod auth;
pub mod models;
mod rate_limit;
pub mod schema;
mod tokens;
mod users;
use std::net::SocketAddr;

const ROUTE_ORIGIN: &str = "/api/v0";

// argon2 needs to allocate a lot of memory for hashing,
// since allocating at runtime is slow and could cause ooms
// we allocate several 'blocks' upfront guarded by mutexs
// and lock one to use whenever we need to hash
// this doubles as a limit on the number of parallel login requests
const HASHER_MEMORY_BLOCKS: usize = 5;

// password hasher parameters
// changing this could stop all users from logging in
const HASHER_MEMORY: u32 = 64_000; // 64MB
const HASHER_ITERATIONS: u32 = 10;
const HASHER_OUTPUT_LEN: u32 = 64;

const HASHER_ALGORITHM: argon2::Algorithm = argon2::Algorithm::Argon2id;
const HASHER_VERSION: argon2::Version = argon2::Version::V0x13;
// todo: unwrap this here when const unwrap gets stabalized
static HASHER_PARAMETERS: Result<argon2::Params, argon2::Error> = argon2::Params::new(
    HASHER_MEMORY,
    HASHER_ITERATIONS,
    1,
    Some(HASHER_OUTPUT_LEN as usize),
);

#[derive(Serialize)]
enum ErrorCode {
    UserExists,
}

enum ApiResponse<T: Serialize> {
    Good(http::StatusCode, Json<T>),
    Bad(http::StatusCode, Json<ErrorInfo>),
    BadNoInfo(http::StatusCode),
}

impl<T: Serialize> IntoResponse for ApiResponse<T> {
    fn into_response(self) -> axum::response::Response {
        match self {
            Self::Bad(code, error) => (code, error).into_response(),
            Self::Good(code, result) => (code, result).into_response(),
            Self::BadNoInfo(code) => code.into_response(),
        }
    }
}

#[derive(Serialize)]
struct ErrorInfo {
    code: ErrorCode,
    message: Option<String>,
}

struct AppState {
    pool: Pool<ConnectionManager<PgConnection>>,
    hasher_memory: [Mutex<Box<[argon2::Block; HASHER_MEMORY as usize]>>; HASHER_MEMORY_BLOCKS],
    request_history: RwLock<HashMap<IpAddr, Mutex<VecDeque<SystemTime>>>>,
}

async fn get_conn_async(
    pool: &Pool<ConnectionManager<PgConnection>>,
) -> PooledConnection<ConnectionManager<PgConnection>> {
    // poor man's .get_async().await
    loop {
        if let Some(c) = pool.try_get() {
            return c;
        }
        yield_now();
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .init();

    dotenv().unwrap();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let app_state: Arc<AppState> = Arc::new(AppState {
        pool: Pool::builder()
            .test_on_check_out(true)
            .build(ConnectionManager::new(database_url))
            .expect("failed to connect to the database"),
        hasher_memory: std::array::from_fn(|_| {
            Mutex::new(
                vec![argon2::Block::new(); HASHER_MEMORY as usize]
                    .into_boxed_slice()
                    .try_into()
                    .unwrap(),
            )
        }),
        request_history: RwLock::new(HashMap::with_capacity(256)),
    });

    let app = Router::new()
        .route(ROUTE_ORIGIN, get(|| async { http::StatusCode::OK }))
        .merge(users::users_router(app_state.clone()))
        .merge(tokens::tokens_router(app_state.clone()))
        .layer(TraceLayer::new_for_http())
        .layer(middleware::from_fn_with_state(
            app_state.clone(),
            rate_limit::rate_limit_basic,
        ));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:23888")
        .await
        .unwrap();

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}
