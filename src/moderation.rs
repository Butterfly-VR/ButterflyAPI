use crate::ApiError;
use crate::AppState;
use crate::ErrorCode;
use crate::ErrorInfo;
use crate::auth;
use crate::models::Moderation;
use crate::models::ObjectDownloadToken;
use crate::models::PermissionsLevel;
use crate::permission_checker;
use crate::schema::moderations;
use crate::schema::object_download_tokens;
use crate::schema::objects;
use crate::schema::tags;
use crate::schema::tokens;
use crate::schema::users;
use axum::Extension;
use axum::extract::State;
use axum::http::StatusCode;
use axum::middleware;
use axum::{Json, Router, routing::get, routing::post};
use diesel::delete;
use diesel::insert_into;
use diesel::prelude::*;
use diesel::update;
use diesel_async::RunQueryDsl;
use serde::Deserialize;
use serde::Serialize;
use std::sync::Arc;
use std::time::Duration;
use std::time::SystemTime;
use uuid::Uuid;

const MODERATION_ROUTE: &str = "/mod";
const MODERATOR_SEARCH_ROUTE: &str = constcat::concat!(MODERATION_ROUTE, "/search");
const MODERATION_MODERATE_OBJECT_ROUTE: &str =
    constcat::concat!(MODERATION_ROUTE, "/moderate_object");
const MODERATION_MODERATE_USER_ROUTE: &str = constcat::concat!(MODERATION_ROUTE, "/moderate_user");
const MODERATION_AQUIRE_OBJECT_TOKEN_ROUTE: &str =
    constcat::concat!(MODERATION_ROUTE, "/object_token");

pub async fn is_moderator() -> StatusCode {
    StatusCode::OK
}

#[derive(Deserialize)]
enum SearchType {
    Objects,
    Users,
}

#[derive(Deserialize)]
pub struct ModeratorSearchRequest {
    search_term: String,
    search_type: SearchType,
    target_id: Option<Uuid>,
}

#[derive(Serialize)]
struct ModerationShortResult {
    id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    creator: Option<Uuid>,
    name: String,
}

impl From<(Uuid, Uuid, String)> for ModerationShortResult {
    fn from((id, creator, name): (Uuid, Uuid, String)) -> Self {
        Self {
            id,
            creator: Some(creator),
            name,
        }
    }
}

impl From<(Uuid, String)> for ModerationShortResult {
    fn from((id, name): (Uuid, String)) -> Self {
        Self {
            id,
            creator: None,
            name,
        }
    }
}

#[derive(Serialize)]
pub struct ModeratorSearchResult {
    results: Vec<ModerationShortResult>,
}

impl From<Vec<ModerationShortResult>> for ModeratorSearchResult {
    fn from(results: Vec<ModerationShortResult>) -> Self {
        Self { results }
    }
}

pub async fn moderator_search(
    State(app_state): State<Arc<AppState>>,
    Json(request): Json<ModeratorSearchRequest>,
) -> Result<Json<ModeratorSearchResult>, ApiError> {
    let mut conn = app_state.pool.get().await?;

    if let Some(target_id) = request.target_id {
        return Ok(Json(
            match request.search_type {
                SearchType::Objects => [objects::table
                    .select((objects::id, objects::creator, objects::name))
                    .distinct_on(objects::id)
                    .filter(objects::id.eq(target_id))
                    .first::<(Uuid, Uuid, String)>(&mut conn)
                    .await?]
                .into_iter()
                .map(ModerationShortResult::from)
                .collect::<Vec<ModerationShortResult>>(),
                SearchType::Users => [users::table
                    .select((users::id, users::username))
                    .filter(users::id.eq(target_id))
                    .first::<(Uuid, String)>(&mut conn)
                    .await?]
                .into_iter()
                .map(ModerationShortResult::from)
                .collect::<Vec<ModerationShortResult>>(),
            }
            .into(),
        ));
    }

    let search_term = &request.search_term;

    Ok(Json(
        match request.search_type {
            SearchType::Objects => objects::table
                .select((objects::id, objects::name))
                .distinct_on(objects::id)
                .left_join(tags::table)
                .inner_join(users::table.on(users::id.eq(objects::creator)))
                .filter(
                    objects::name
                        .like(format!("%{search_term}%"))
                        .or(objects::description.like(format!("%{search_term}%")))
                        .or(tags::tag.like(format!("%{search_term}%")))
                        .or(users::username.like(format!("%{search_term}%"))),
                )
                .filter(objects::delete_at.is_null())
                .limit(5000)
                .load::<(Uuid, String)>(&mut conn)
                .await?
                .into_iter()
                .map(ModerationShortResult::from)
                .collect::<Vec<ModerationShortResult>>(),
            SearchType::Users => users::table
                .select((users::id, users::username))
                .distinct_on(users::id)
                .filter(users::username.like(format!("%{search_term}%")))
                .limit(100)
                .load::<(Uuid, String)>(&mut conn)
                .await?
                .into_iter()
                .map(ModerationShortResult::from)
                .collect::<Vec<ModerationShortResult>>(),
        }
        .into(),
    ))
}

#[derive(Deserialize)]
enum ObjectActions {
    Remove,
    Verify,
}

#[derive(Deserialize)]
pub struct ModerateObjectRequest {
    target: Uuid,
    action: ObjectActions,
}

pub async fn moderate_object(
    State(app_state): State<Arc<AppState>>,
    Json(request): Json<ModerateObjectRequest>,
) -> Result<(), ApiError> {
    let mut conn = app_state.pool.get().await?;
    match request.action {
        ObjectActions::Remove => {
            update(objects::table)
                .filter(objects::id.eq(request.target))
                .set(objects::delete_at.eq(SystemTime::now()))
                .execute(&mut conn)
                .await?;
            Ok(())
        }
        ObjectActions::Verify => {
            update(objects::table)
                .filter(objects::id.eq(request.target))
                .set(objects::verified.eq(true))
                .execute(&mut conn)
                .await?;
            Ok(())
        }
    }
}

#[derive(Deserialize)]
pub enum UserModerationType {
    Ban,
}

#[derive(Deserialize)]
pub struct UserModerationRequest {
    pub target: Uuid,
    pub moderation_type: UserModerationType,
    pub expiry: Option<SystemTime>,
    pub reason: Option<String>,
}

pub async fn moderate_user(
    State(app_state): State<Arc<AppState>>,
    Extension(user_id): Extension<Uuid>,
    Json(request): Json<UserModerationRequest>,
) -> Result<(), ApiError> {
    let mut conn = app_state.pool.get().await?;

    match request.moderation_type {
        UserModerationType::Ban => {
            insert_into(moderations::table)
                .values(Moderation {
                    id: Uuid::new_v4(),
                    target: request.target,
                    moderator: Some(user_id),
                    type_: request.moderation_type as i16,
                    expires: request.expiry,
                    details: request.reason,
                })
                .execute(&mut conn)
                .await?;
            delete(tokens::table)
                .filter(tokens::user.eq(request.target))
                .execute(&mut conn)
                .await?;
            Ok(())
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ObjectTokenResponse {
    token: Uuid,
    object_id: Uuid,
    creator: Uuid,
}

pub async fn aquire_object_token(
    State(app_state): State<Arc<AppState>>,
) -> Result<Json<ObjectTokenResponse>, ApiError> {
    const TOKEN_EXPIRY: Duration = Duration::from_mins(15);

    let mut conn = app_state.pool.get().await?;

    let Some((object_id, creator)) = (objects::table)
        .select((objects::id, objects::creator))
        .filter(objects::verified.eq(false))
        .filter(
            objects::id.ne_all(
                object_download_tokens::table
                    .select(object_download_tokens::token)
                    .filter(object_download_tokens::expiry.gt(SystemTime::now())),
            ),
        )
        .first(&mut conn)
        .await
        .optional()?
    else {
        return Err(ApiError::WithResponse(
            StatusCode::NOT_FOUND,
            Json(ErrorInfo {
                error_code: ErrorCode::DosentExist,
                error_message: None,
            }),
        ));
    };

    let id = Uuid::new_v4();

    insert_into(object_download_tokens::table)
        .values(ObjectDownloadToken {
            token: id,
            object_id: object_id,
            used: false,
            expiry: SystemTime::now() + TOKEN_EXPIRY,
        })
        .execute(&mut conn)
        .await?;
    return Ok(Json(ObjectTokenResponse {
        token: id,
        object_id,
        creator,
    }));
}

pub fn moderation_router(app_state: Arc<AppState>) -> Router {
    Router::new()
        .route(MODERATION_ROUTE, get(is_moderator))
        .route(MODERATOR_SEARCH_ROUTE, post(moderator_search))
        .route(MODERATION_MODERATE_OBJECT_ROUTE, post(moderate_object))
        .route(MODERATION_MODERATE_USER_ROUTE, post(moderate_user))
        .route(
            MODERATION_AQUIRE_OBJECT_TOKEN_ROUTE,
            get(aquire_object_token),
        )
        .layer(middleware::from_fn_with_state(
            app_state.clone(),
            auth::check_auth,
        ))
        .layer(middleware::from_fn_with_state(
            app_state.clone(),
            |state, req, next| {
                permission_checker::check_permissions(
                    vec![PermissionsLevel::Moderator, PermissionsLevel::Admin],
                    state,
                    req,
                    next,
                )
            },
        ))
        .with_state(app_state)
}
