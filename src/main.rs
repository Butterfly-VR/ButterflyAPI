use argon2::Argon2;
use axum::{Router, http, middleware, routing::get};
use diesel::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool};
use dotenvy::dotenv;
use std::{env, sync::Arc};
use tower_http::trace::TraceLayer;
mod auth;
pub mod models;
mod rate_limit;
pub mod schema;
mod users;

struct AppState {
    pool: Pool<ConnectionManager<PgConnection>>,
    password_hasher: Argon2<'static>,
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
            .expect("failed to connect to database"),
        password_hasher: Argon2::new(
            argon2::Algorithm::Argon2id,
            argon2::Version::V0x13,
            argon2::Params::new(32000, 4, 1, Some(32)).unwrap(),
        ),
    });
    let app = Router::new()
        .route("/", get(|| async { http::StatusCode::IM_A_TEAPOT }))
        .merge(users::users_router(app_state))
        .layer(TraceLayer::new_for_http())
        .layer(middleware::from_fn(rate_limit::check_rate_limits));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:23888")
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}
