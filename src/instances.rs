use crate::ApiError;
use crate::AppState;
use crate::auth;
use crate::gameserver_handler::allocate_gameserver;
use crate::models::Instance;
use crate::schema::instances;
use crate::schema::users;
use axum::Extension;
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
use diesel_async::{AsyncConnection, RunQueryDsl};
use rand::TryRng;
use rand::rngs::SysRng;
use serde::Deserialize;
use serde::Serialize;
use std::sync::Arc;
use uuid::Uuid;

const INSTANCES_ROUTE: &str = "/instances";
const INSTANCE_SEARCH_ROUTE: &str = constcat::concat!(INSTANCES_ROUTE, "/search");
const INSTANCE_ID_ROUTE: &str = constcat::concat!(INSTANCES_ROUTE, "/{id}");
const INSTANCE_JOIN_ROUTE: &str = constcat::concat!(INSTANCE_ID_ROUTE, "/join");

#[derive(Deserialize)]
pub struct InstanceCreation {
    world: Uuid,
    name: String,
    max_players: i16,
    publicity: i16,
    anyone_can_invite: bool,
    is_gameserver: bool,
}

#[derive(Serialize)]
pub struct InstanceCreationResult {
    id: Uuid,
}

pub async fn create_instance(
    State(state): State<Arc<AppState>>,
    Json(instance_details): Json<InstanceCreation>,
) -> Result<Json<InstanceCreationResult>, ApiError> {
    let mut conn = state.pool.get().await?;

    let id = Uuid::new_v4();

    let mut instance_token: [u8; 64] = [0; 64];
    SysRng.try_fill_bytes(&mut instance_token)?;

    let addr = allocate_gameserver(
        state.clone(),
        id,
        instance_token,
        instance_details.world,
        instance_details.is_gameserver,
    )
    .await?;

    let instance: Instance = Instance {
        id,
        server_token: instance_token.to_vec(),
        world: instance_details.world,
        name: instance_details.name,
        max_players: instance_details.max_players,
        publicity: instance_details.publicity,
        anyone_can_invite: instance_details.anyone_can_invite,
        is_gameserver: instance_details.is_gameserver,
        ip: addr.ip().into(),
        port: addr.port().into(),
    };

    insert_into(instances::table)
        .values(instance)
        .execute(&mut conn)
        .await?;

    Ok(Json(InstanceCreationResult { id }))
}

#[derive(Deserialize)]
pub struct InstanceSearch {
    world: Uuid,
    #[serde(default)]
    is_full: Option<bool>,
    #[serde(default)]
    is_empty: Option<bool>,
    #[serde(default)]
    is_gameserver: Option<bool>,
}

#[derive(Serialize)]
pub struct InstanceSearchResult {
    instances: Vec<InstanceInfo>,
}

#[derive(Serialize)]
pub struct InstanceInfo {
    pub id: Uuid,
    pub world: Uuid,
    pub name: String,
    pub max_players: i16,
    pub current_players: i16,
    pub publicity: i16,
    pub anyone_can_invite: bool,
    pub is_gameserver: bool,
    pub ip: String,
    pub port: u16,
}

impl InstanceInfo {
    pub fn new(instance: Instance, current_players: i16) -> Self {
        Self {
            id: instance.id,
            world: instance.world,
            name: instance.name,
            max_players: instance.max_players,
            current_players,
            publicity: instance.publicity,
            anyone_can_invite: instance.anyone_can_invite,
            is_gameserver: instance.is_gameserver,
            ip: instance.ip.to_string(),
            port: instance.port.try_into().unwrap_or_default(),
        }
    }
}

impl From<Vec<InstanceInfo>> for InstanceSearchResult {
    fn from(instances: Vec<InstanceInfo>) -> Self {
        Self { instances }
    }
}

pub async fn search_instances(
    State(state): State<Arc<AppState>>,
    Json(search): Json<InstanceSearch>,
) -> Result<Json<InstanceSearchResult>, ApiError> {
    let mut conn = state.pool.get().await?;

    conn.transaction(async |conn| {
        {
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
                        .map(|(instance, current_players): (Instance, i64)| {
                            InstanceInfo::new(instance, current_players as i16)
                        })
                        .collect::<Vec<InstanceInfo>>()
                        .into()
                })
                .map(Json)
        }
    })
    .await
}

pub async fn get_instance(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<InstanceInfo>, ApiError> {
    let mut conn = state.pool.get().await?;
    instances::table
        .left_join(users::table)
        .group_by(instances::id)
        .select((Instance::as_select(), count(users::id.nullable())))
        .filter(instances::id.eq(id))
        .first::<(Instance, i64)>(&mut conn)
        .await
        .map_err(ApiError::from)
        .map(|(instance, current_players)| InstanceInfo::new(instance, current_players as i16))
        .map(Json)
}

#[derive(Serialize)]
pub struct InstanceJoinInfo {
    pub ip: String,
    pub port: u16,
    pub identifier: Vec<u8>,
}

pub async fn join_instance(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Extension(user_id): Extension<Uuid>,
) -> Result<Json<InstanceJoinInfo>, ApiError> {
    let mut conn = state.pool.get().await?;

    conn.transaction(async |mut conn| {
        {
            let Some(instance) = instances::table
                .select(Instance::as_select())
                .for_no_key_update()
                .filter(instances::id.eq(id))
                .first(&mut conn)
                .await
                .optional()?
            else {
                return Err(ApiError::WithCode(StatusCode::NOT_FOUND));
            };

            // todo: check for instance privacy, blocks, etc

            let mut identifier = [0u8; 8];
            SysRng.try_fill_bytes(&mut identifier)?;

            diesel::update(users::table.find(user_id))
                .set((
                    users::instance.eq(instance.id),
                    users::identifier.eq(identifier),
                ))
                .execute(&mut conn)
                .await?;

            Ok(InstanceJoinInfo {
                ip: instance.ip.to_string(),
                port: instance.port as u16,
                identifier: identifier.to_vec(),
            })
        }
    })
    .await
    .map(Json)
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
