use crate::ApiError;
use crate::AppState;
use crate::auth;
use crate::gameserver_handler::ConnectTokenRetrievalError;
use crate::gameserver_handler::allocate_gameserver;
use crate::gameserver_handler::get_connect_token;
use crate::models::*;
use crate::schema::instances;
use crate::schema::users;
use axum::extract::Path;
use axum::extract::State;
use axum::http::StatusCode;
use axum::middleware;
use axum::{Json, Router, routing::get, routing::post};
use diesel::dsl::count;
use diesel::dsl::sql;
use diesel::insert_into;
use diesel::prelude::*;
use diesel::sql_types::BigInt;
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::{AsyncConnection, RunQueryDsl};
use rand::TryRngCore;
use rand::rngs::OsRng;
use serde::Deserialize;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, warn};
use uuid::Uuid;

const INSTANCES_ROUTE: &str = "/instances";
const INSTANCE_ID_ROUTE: &str = constcat::concat!(INSTANCES_ROUTE, "/{id}");
const INSTANCE_JOIN_ROUTE: &str = constcat::concat!(INSTANCE_ID_ROUTE, "/join");
const INSTANCE_SEARCH_ROUTE: &str = constcat::concat!(INSTANCES_ROUTE, "/search");

#[derive(Deserialize)]
pub struct InstanceCreation {
    world: Uuid,
    name: String,
    max_players: i16,
    publicity: i16,
    anyone_can_invite: bool,
    is_gameserver: bool,
}

pub async fn create_instance(
    State(state): State<Arc<AppState>>,
    Json(instance_details): Json<InstanceCreation>,
) -> Result<(), ApiError> {
    let mut conn = state.pool.get().await?;

    let id = Uuid::new_v4();

    let mut instance_token: [u8; 64] = [0; 64];
    OsRng.try_fill_bytes(&mut instance_token)?;

    let instance: Instance = Instance {
        id,
        server_token: instance_token.to_vec(),
        world: instance_details.world,
        name: instance_details.name,
        max_players: instance_details.max_players,
        publicity: instance_details.publicity,
        anyone_can_invite: instance_details.anyone_can_invite,
        is_gameserver: instance_details.is_gameserver,
        last_used_client_token: [0_u8; 64].to_vec(),
    };

    insert_into(instances::table)
        .values(instance)
        .execute(&mut conn)
        .await?;
    allocate_gameserver(
        state.clone(),
        id,
        instance_token,
        instance_details.world,
        instance_details.is_gameserver,
    )
    .await
}

#[derive(Deserialize)]
pub struct InstanceSearch {
    world: Uuid,
    is_full: Option<bool>,
    is_empty: Option<bool>,
    is_gameserver: Option<bool>,
}

pub async fn search_instances(
    State(state): State<Arc<AppState>>,
    Json(search): Json<InstanceSearch>,
) -> Result<Json<Vec<Instance>>, ApiError> {
    let mut conn = state.pool.get().await?;

    conn.transaction(|conn| {
        async move {
            let mut query = instances::table
                .left_join(users::table)
                .group_by(instances::id)
                .select((Instance::as_select(), count(users::id.nullable())))
                .filter(instances::world.eq(search.world))
                .limit(100)
                .into_boxed();

            if let Some(is_gameserver) = search.is_gameserver {
                query = query.filter(instances::is_gameserver.eq(is_gameserver));
            }

            if let Some(is_full) = search.is_full {
                if is_full {
                    query = query.having(
                        // im probably holding it wrong but i cant find any other way to cast smallint to bigint
                        count(users::id).eq(sql::<BigInt>("CAST(")
                            .bind(instances::max_players)
                            .sql(" AS BIGINT)")),
                    );
                } else {
                    query = query.having(
                        // im probably holding it wrong but i cant find any other way to cast smallint to bigint
                        count(users::id).ne(sql::<BigInt>("CAST(")
                            .bind(instances::max_players)
                            .sql(" AS BIGINT)")),
                    );
                }
            }

            if let Some(is_empty) = search.is_empty {
                if is_empty {
                    query = query.having(count(users::id).eq(0));
                } else {
                    query = query.having(count(users::id).ne(0));
                }
            }

            query
                .load(conn)
                .await
                .map_err(ApiError::from)
                .map(|x| {
                    x.into_iter()
                        .map(|(instance, _): (Instance, i64)| instance)
                        .collect() // extract the instances and ignore the player count
                })
                .map(Json)
        }
        .scope_boxed()
    })
    .await
}

pub async fn get_instance(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<Instance>, ApiError> {
    let mut conn = state.pool.get().await?;
    instances::table
        .select(Instance::as_select())
        .filter(instances::id.eq(id))
        .first::<Instance>(&mut conn)
        .await
        .map_err(ApiError::from)
        .map(Json)
}

pub async fn join_instance(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<u8>>, ApiError> {
    const DUPLICATE_TOKEN_RETRY_TIME: Duration = Duration::from_secs(1);
    const MAX_RETY_ATTEMPTS: usize = 5;

    let mut conn = state.pool.get().await?;
    let state = state.clone();

    conn.transaction(|mut conn| {
        async move {
            let Some(instance) = instances::table
                .select(Instance::as_select())
                .filter(instances::id.eq(id))
                .first(&mut conn)
                .await
                .optional()?
            else {
                return Err(ApiError::WithCode(StatusCode::NOT_FOUND));
            };
            for _ in 0..MAX_RETY_ATTEMPTS {
                match get_connect_token(state.clone(), id).await {
                    Ok(token) => {
                        if token == instance.last_used_client_token {
                            info!(
                                "got duplicate token, if this keeps happening we may need to rework this handling"
                            );
                            sleep(DUPLICATE_TOKEN_RETRY_TIME).await;
                            continue;
                        } else {
                            return Ok(Json(token));
                        }
                    }
                    Err(err) => match err {
                        ConnectTokenRetrievalError::GameserverClosed => {
                            return Err(ApiError::WithCode(StatusCode::NOT_FOUND));
                        }
                        ConnectTokenRetrievalError::GameserverNotReady => {
                            return Err(ApiError::WithCode(StatusCode::SERVICE_UNAVAILABLE));
                        }
                        ConnectTokenRetrievalError::Generic(err) => return Err(err),
                    },
                }
            }
            // must of hit max reties?
            warn!("ran out of attempts retriving connect token");
            Err(ApiError::WithCode(StatusCode::INTERNAL_SERVER_ERROR))
        }
        .scope_boxed()
    })
    .await
}

pub fn instances_router(app_state: Arc<AppState>) -> Router {
    Router::new()
        .route(INSTANCES_ROUTE, post(create_instance))
        .route(INSTANCE_SEARCH_ROUTE, post(search_instances))
        .route(INSTANCE_ID_ROUTE, get(get_instance))
        .route(INSTANCE_JOIN_ROUTE, get(join_instance))
        .layer(middleware::from_fn_with_state(
            app_state.clone(),
            auth::check_auth,
        ))
        .with_state(app_state)
}
