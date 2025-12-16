use crate::hash::HASHER_MEMORY;
use axum::Json;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{Router, http, middleware, routing::get};
use bb8::Pool;
use diesel_async::AsyncPgConnection;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use dotenvy::dotenv;
use serde::Serialize;
use std::collections::{HashMap, VecDeque};
use std::error::Error;
use std::net::IpAddr;
use std::time::SystemTime;
use std::{env, sync::Arc};
use tokio::sync::{Mutex, RwLock};
use tower_http::trace::TraceLayer;
mod auth;
mod hash;
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
// there isnt much point in having this more than the number of
// hardware threads, since it wastes memory and can cause timing issues
const HASHER_MEMORY_BLOCKS: usize = 1;

#[derive(Serialize)]
enum ErrorCode {
    UserAlreadyExists,
    UserDosentExist,
}

enum ApiError {
    WithResponse(http::StatusCode, Json<ErrorInfo>),
    WithCode(http::StatusCode),
}

impl<T: Error> From<T> for ApiError {
    fn from(_: T) -> Self {
        Self::WithCode(StatusCode::INTERNAL_SERVER_ERROR)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        match self {
            Self::WithResponse(code, error) => (code, error).into_response(),
            Self::WithCode(code) => code.into_response(),
        }
    }
}

#[derive(Serialize)]
struct ErrorInfo {
    error_code: ErrorCode,
    error_message: Option<String>,
}

struct AppState {
    // todo: optimise some connections to readonly
    //readonly_pool: Pool<AsyncDieselConnectionManager<AsyncPgConnection>>,
    pool: Pool<AsyncDieselConnectionManager<AsyncPgConnection>>,
    s3_client: aws_sdk_s3::Client,
    hasher_memory: [Mutex<Vec<argon2::Block>>; HASHER_MEMORY_BLOCKS],
    request_history: RwLock<HashMap<IpAddr, Mutex<VecDeque<SystemTime>>>>,
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
            .build(AsyncDieselConnectionManager::new(database_url))
            .await
            .expect("failed to connect to the database"),
        s3_client: aws_sdk_s3::Client::new(&aws_config::load_from_env().await),
        hasher_memory: std::array::from_fn(|_| {
            Mutex::new(vec![argon2::Block::new(); HASHER_MEMORY as usize])
        }),
        request_history: RwLock::new(HashMap::with_capacity(256)),
    });

    let app = Router::new()
        .route(ROUTE_ORIGIN, get(|| async { http::StatusCode::OK }))
        .nest(ROUTE_ORIGIN, users::users_router(app_state.clone()))
        .nest(ROUTE_ORIGIN, tokens::tokens_router(app_state.clone()))
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
