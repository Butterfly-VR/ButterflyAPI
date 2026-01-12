use crate::ApiError;
use crate::AppState;
use crate::auth;
use crate::models::Object;
use crate::models::PublicUserInfo;
use crate::schema::objects;
use crate::schema::tags;
use crate::schema::users;
use axum::extract::Path;
use axum::extract::State;
use axum::http::StatusCode;
use axum::middleware;
use axum::{Json, Router, routing::get};
use diesel::prelude::*;
use diesel_async::AsyncPgConnection;
use diesel_async::RunQueryDsl;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

const SEARCH_ROUTE: &str = "/search/{query}";

#[derive(Serialize)]
pub struct SearchResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    users: Option<Vec<PublicUserInfo>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    worlds: Option<Vec<Object>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    avatars: Option<Vec<Object>>,
}

#[derive(Debug, Clone, Copy)]
pub enum FilterObjectTypes {
    World = 0,
    Avatar = 1,
    User = 2,
}

pub enum SortTypes {
    Name,
    CreatedAt,
    UpdatedAt,
    WeeklyUses,
}

pub enum Filter {
    Invalid,
    Is(FilterObjectTypes),
    Owner(Uuid),
    SortBy(SortTypes),
}

fn parse_filters(filters_map: HashMap<&str, &str>) -> Vec<Filter> {
    let mut filters = Vec::with_capacity(filters_map.len());

    for filter in filters_map.into_iter() {
        match filter {
            ("is", type_str) => match type_str {
                "user" => filters.push(Filter::Is(FilterObjectTypes::User)),
                "world" => filters.push(Filter::Is(FilterObjectTypes::World)),
                "avatar" => filters.push(Filter::Is(FilterObjectTypes::Avatar)),
                _ => filters.push(Filter::Invalid),
            },

            ("owner", id) => {
                if let Ok(user) = Uuid::parse_str(id) {
                    filters.push(Filter::Owner(user));
                } else {
                    filters.push(Filter::Invalid);
                }
            }

            ("sort", sort) => match sort {
                "name" => filters.push(Filter::SortBy(SortTypes::Name)),
                "created_at" => filters.push(Filter::SortBy(SortTypes::CreatedAt)),
                "updated_at" => filters.push(Filter::SortBy(SortTypes::UpdatedAt)),
                "weekly_uses" => filters.push(Filter::SortBy(SortTypes::WeeklyUses)),
                _ => filters.push(Filter::Invalid),
            },

            _ => filters.push(Filter::Invalid),
        }
    }
    filters
}

pub async fn search(
    State(app_state): State<Arc<AppState>>,
    Path(query): Path<String>,
) -> Result<Json<SearchResult>, ApiError> {
    // todo: replace unwraps with error handling
    dbg!(&query);
    let (term, filters) = query
        .split_once('&')
        .ok_or(ApiError::WithCode(StatusCode::BAD_REQUEST))?;

    let filters = filters
        .split(',')
        .filter(|x| !x.is_empty())
        .collect::<Vec<&str>>();

    let filters: HashMap<&str, &str> = filters
        .into_iter()
        .filter_map(|x| x.split_once(":"))
        .collect();

    let filters = parse_filters(filters);

    let mut conn = app_state.pool.get().await?;

    let mut search_result = SearchResult {
        users: None,
        worlds: None,
        avatars: None,
    };

    for filter in filters.iter() {
        match filter {
            Filter::Is(FilterObjectTypes::User) => {
                search_result.users = Some(search_users(&filters, term, &mut conn).await);
                break;
            }
            Filter::Is(FilterObjectTypes::World) => {
                search_result.worlds =
                    Some(search_objects(FilterObjectTypes::World, &filters, term, &mut conn).await);
                break;
            }
            Filter::Is(FilterObjectTypes::Avatar) => {
                search_result.avatars = Some(
                    search_objects(FilterObjectTypes::Avatar, &filters, term, &mut conn).await,
                );
                break;
            }
            _ => {}
        }
    }

    Ok(Json(search_result))
}

pub async fn search_objects(
    object_type: FilterObjectTypes,
    filters: &[Filter],
    search_term: &str,
    conn: &mut AsyncPgConnection,
) -> Vec<Object> {
    let mut query = objects::table
        .select(Object::as_select())
        .filter(objects::name.like(format!("%{}%", search_term)))
        .or_filter(objects::description.like(format!("%{}%", search_term)))
        .left_join(tags::table)
        .or_filter(tags::tag.eq(search_term))
        .left_join(users::table.on(users::id.eq(objects::creator)))
        .or_filter(users::username.like(format!("%{}%", search_term)))
        .limit(500)
        .into_boxed();

    query = query.filter(objects::object_type.eq(object_type as i16));

    for filter in filters {
        match filter {
            Filter::Owner(owner) => {
                query = query.filter(users::id.eq(owner));
            }
            Filter::SortBy(sort_type) => match sort_type {
                SortTypes::Name => {
                    query = query.order(objects::name.asc());
                }
                SortTypes::CreatedAt => {
                    query = query.order(objects::created_at.desc());
                }
                _ => {}
            },
            _ => {}
        }
    }
    query.load(conn).await.unwrap()
}

pub async fn search_users(
    filters: &[Filter],
    search_term: &str,
    conn: &mut AsyncPgConnection,
) -> Vec<PublicUserInfo> {
    let mut query = users::table
        .select(PublicUserInfo::as_select())
        .filter(users::username.like(format!("%{}%", search_term)))
        .limit(100)
        .into_boxed();
    for filter in filters {
        match filter {
            Filter::SortBy(sort_type) => match sort_type {
                SortTypes::Name => {
                    query = query.order(users::username.asc());
                }
                _ => {}
            },
            _ => {}
        }
    }
    query.load(conn).await.unwrap()
}

pub fn search_router(app_state: Arc<AppState>) -> Router {
    Router::new()
        .route(SEARCH_ROUTE, get(search))
        .layer(middleware::from_fn_with_state(
            app_state.clone(),
            auth::check_auth,
        ))
        .with_state(app_state)
}
