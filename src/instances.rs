use crate::ApiError;
use crate::AppState;
use crate::auth;
use crate::models::*;
use crate::schema::instances;
use crate::schema::users;
use axum::extract::State;
use axum::middleware;
use axum::{Json, Router, routing::get, routing::post};
use diesel::dsl::count;
use diesel::dsl::sql;
use diesel::prelude::*;
use diesel::sql_types::BigInt;
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::{AsyncConnection, RunQueryDsl};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

const INSTANCES_ROUTE: &str = "/instances";
const INSTANCE_ID_ROUTE: &str = constcat::concat!(INSTANCES_ROUTE, "/{id}");
const INSTANCE_JOIN_ROUTE: &str = constcat::concat!(INSTANCE_ID_ROUTE, "/join");
const INSTANCE_SEARCH_ROUTE: &str = constcat::concat!(INSTANCES_ROUTE, "/search");

#[derive(Deserialize)]
struct InstanceSearch {
    world: Uuid,
    is_full: Option<bool>,
    is_empty: Option<bool>,
    is_gameserver: Option<bool>,
}

#[axum::debug_handler]
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
                        .collect()
                })
                .map(Json)
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
