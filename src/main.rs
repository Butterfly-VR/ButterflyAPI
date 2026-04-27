#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

use crate::hash::HASHER_MEMORY;
use axum::Json;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{Router, http, routing::get};
use bb8::Pool;
use diesel_async::AsyncPgConnection;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use dotenvy::dotenv;
use serde::Serialize;
use std::error::Error;
use std::{env, sync::Arc};
use tokio::sync::Mutex;
use tower_http::trace::TraceLayer;
mod auth;
mod email;
mod gameserver_handler;
mod hash;
mod instance_api;
mod instances;
mod kube_resources;
pub mod models;
mod objects;
pub mod schema;
mod search;
mod tokens;
mod users;
use std::net::SocketAddr;

const ROUTE_ORIGIN: &str = "/api/v0";
const COFFEE_ORIGIN: &str = "/api/v0/coffee";

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
    AlreadyExists,
    DosentExist,
    InsufficientPermissions,
    BadRequestLength,
    InvalidRequest,
}

enum ApiError {
    WithResponse(http::StatusCode, Json<ErrorInfo>),
    WithCode(http::StatusCode),
}

impl<T: Error> From<T> for ApiError {
    fn from(error: T) -> Self {
        tracing::error!("{:?}", error);
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
    #[serde(skip_serializing_if = "Option::is_none")]
    error_message: Option<String>,
}

struct AppState {
    // todo: optimise some connections to readonly
    //readonly_pool: Pool<AsyncDieselConnectionManager<AsyncPgConnection>>,
    pool: Pool<AsyncDieselConnectionManager<AsyncPgConnection>>,
    s3_client: aws_sdk_s3::Client,
    kube_client: kube::Client,
    hasher_memory: [Mutex<Vec<argon2::Block>>; HASHER_MEMORY_BLOCKS],
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .init();

    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .unwrap();

    let _ = dotenv();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let app_state: Arc<AppState> = Arc::new(AppState {
        pool: Pool::builder()
            .build(AsyncDieselConnectionManager::new(database_url))
            .await
            .expect("failed to connect to the database"),
        s3_client: aws_sdk_s3::Client::new(&aws_config::load_from_env().await),
        kube_client: kube::Client::try_default()
            .await
            .expect("failed to connect to the kube api"),
        hasher_memory: std::array::from_fn(|_| {
            Mutex::new(vec![argon2::Block::new(); HASHER_MEMORY as usize])
        }),
    });

    let app = Router::new()
        .route(ROUTE_ORIGIN, get(|| async { http::StatusCode::OK }))
        .route(COFFEE_ORIGIN, get(|| async { http::StatusCode::IM_A_TEAPOT }))
        .nest(ROUTE_ORIGIN, users::users_router(app_state.clone()))
        .nest(ROUTE_ORIGIN, tokens::tokens_router(app_state.clone()))
        .nest(ROUTE_ORIGIN, objects::objects_router(app_state.clone()))
        .nest(ROUTE_ORIGIN, search::search_router(app_state.clone()))
        .nest(ROUTE_ORIGIN, instances::instances_router(app_state.clone()))
        .nest(
            ROUTE_ORIGIN,
            instance_api::instance_api_router(app_state.clone()),
        )
        .layer(TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:80").await.unwrap();

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}
