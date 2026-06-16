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
use std::collections::HashMap;
use std::env::args;
use std::error::Error;
use std::fs;
use std::hint::black_box;
use std::sync::Mutex;
use std::sync::atomic::AtomicU64;
use std::time::{Duration, Instant, SystemTime};
use std::{env, sync::Arc};
use tokio::sync::RwLock;
use tower_http::trace::TraceLayer;
use uuid::Uuid;
mod auth;
mod email;
mod file_cache;
mod gameserver_handler;
mod hash;
mod instance_api;
mod instances;
mod jobs;
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
const HEALTH_CHECK_ORIGIN: &str = "/api/v0/health";
// should be equal to the size of the emptydir
const CACHE_SIZE_KB: u64 = 1024 * 1024 * 10;

// argon2 needs to allocate a lot of memory for hashing,
// since allocating at runtime is slow and could cause ooms
// we allocate several 'blocks' upfront guarded by mutexs
// and lock one to use whenever we need to hash
// this shouldnt be more than the number of available threads,
// since it wastes memory with no benefit
const HASHER_MEMORY_BLOCKS: usize = 2;

#[derive(Serialize)]
enum ErrorCode {
    AlreadyExists,
    DosentExist,
    InsufficientPermissions,
    BadRequestLength,
    InvalidRequest,
    InsufficientSpace,
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
    user_rate_limits: RwLock<HashMap<Uuid, RateLimitInfo>>,
    object_cache: moka::future::Cache<Uuid, CacheEntry>,
    image_cache: moka::future::Cache<Uuid, CacheEntry>,
}

#[derive(Debug, Clone)]
pub struct CacheEntry {
    cached_at: SystemTime,
    size_kb: u32,
    file: Arc<fs::File>,
}

pub struct RateLimitInfo {
    total_requests: AtomicU64,
    next_reset: Instant,
}

impl RateLimitInfo {
    fn new(reset_interval: Duration) -> Self {
        Self {
            total_requests: AtomicU64::new(0),
            next_reset: Instant::now() + reset_interval,
        }
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .unwrap();

    let _ = dotenv();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let pool = Pool::builder()
        .build(AsyncDieselConnectionManager::new(database_url))
        .await
        .expect("failed to connect to the database");
    let s3_client = aws_sdk_s3::Client::new(&aws_config::load_from_env().await);

    if args().find(|arg| arg == "--run_jobs").is_some() {
        jobs::run_all_jobs(pool, s3_client).await;
        return;
    }

    let kube_client = kube::Client::try_default()
        .await
        .expect("failed to connect to the kube api");

    let app_state: Arc<AppState> = Arc::new(AppState {
        pool,
        s3_client,
        kube_client,
        hasher_memory: std::array::from_fn(|_| {
            Mutex::new(vec![argon2::Block::new(); HASHER_MEMORY as usize])
        }),
        user_rate_limits: RwLock::new(HashMap::with_capacity(1024)),
        object_cache: moka::future::Cache::builder()
            .max_capacity(CACHE_SIZE_KB)
            .initial_capacity(1000)
            // max of 1000 entries
            .weigher(|_, v: &CacheEntry| v.size_kb.min((CACHE_SIZE_KB / 1000) as u32))
            .build(),
        image_cache: moka::future::Cache::builder()
            .max_capacity(CACHE_SIZE_KB)
            .initial_capacity(1000)
            // max of 1000 entries
            .weigher(|_, v: &CacheEntry| v.size_kb.min((CACHE_SIZE_KB / 1000) as u32))
            .build(),
    });

    let health_check_state = app_state.clone();

    let app = Router::new()
        .route(ROUTE_ORIGIN, get(async move || http::StatusCode::OK))
        .route(
            HEALTH_CHECK_ORIGIN,
            get(async move || {
                // this route is used as a health check
                // so we should check the database connection and clients
                let _ = black_box(health_check_state.pool.get().await.unwrap());
                let _ = black_box(
                    health_check_state
                        .s3_client
                        .list_buckets()
                        .send()
                        .await
                        .unwrap(),
                );
                let _ = black_box(
                    health_check_state
                        .kube_client
                        .list_api_groups()
                        .await
                        .unwrap(),
                );
                http::StatusCode::OK
            }),
        )
        .route(
            COFFEE_ORIGIN,
            // this is also used as a liveness check
            get(|| async { http::StatusCode::IM_A_TEAPOT }),
        )
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
