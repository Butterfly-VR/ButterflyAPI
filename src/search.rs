use crate::ApiError;
use crate::AppState;
use crate::auth;
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
    users: Option<Vec<(Uuid, String)>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    worlds: Option<Vec<(Uuid, String)>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    avatars: Option<Vec<(Uuid, String)>>,
}

pub enum FilterObjectTypes {
    User,
    World,
    Avatar,
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
    // split query to search term and filters
    let (term, filters) = query
        .split_once('&')
        .ok_or(ApiError::WithCode(StatusCode::BAD_REQUEST))?;

    let filters = filters.split(',').collect::<Vec<&str>>();

    let filters: HashMap<&str, &str> = filters
        .into_iter()
        .map(|x| x.split_once(":").unwrap())
        .collect();

    let mut conn = app_state.pool.get().await?;

    let filters = parse_filters(filters);

    let mut search_users = false;
    let mut search_worlds = false;
    let mut search_avatars = false;

    for filter in filters.iter() {
        match filter {
            Filter::Is(FilterObjectTypes::User) => search_users = true,
            Filter::Is(FilterObjectTypes::World) => search_worlds = true,
            Filter::Is(FilterObjectTypes::Avatar) => search_avatars = true,
            _ => {}
        }
    }

    if !search_users && !search_worlds && !search_avatars {
        search_users = true;
        search_worlds = true;
        search_avatars = true;
    }

    let mut search_result = SearchResult {
        users: None,
        worlds: None,
        avatars: None,
    };

    if search_users {
        search_result.users =
            Some(perform_search(FilterObjectTypes::User, &filters, term, &mut conn).await);
    }

    if search_worlds {
        search_result.worlds =
            Some(perform_search(FilterObjectTypes::World, &filters, term, &mut conn).await);
    }

    if search_avatars {
        search_result.avatars =
            Some(perform_search(FilterObjectTypes::Avatar, &filters, term, &mut conn).await);
    }

    Ok(Json(search_result))
}

pub async fn perform_search(
    target: FilterObjectTypes,
    filters: &[Filter],
    search_term: &str,
    conn: &mut AsyncPgConnection,
) -> Vec<(Uuid, String)> {
    match target {
        FilterObjectTypes::User => {
            let mut query = users::table
                .select((users::id, users::username))
                .filter(users::username.like(format!("%{}%", search_term)))
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
            query.load::<(Uuid, String)>(conn).await.unwrap()
        }
        _ => {
            let mut query = objects::table
                .select((objects::id, objects::name))
                .filter(objects::name.like(format!("%{}%", search_term)))
                .or_filter(objects::description.like(format!("%{}%", search_term)))
                .left_join(tags::table)
                .or_filter(tags::tag.eq(search_term))
                .left_join(users::table.on(users::id.eq(objects::creator)))
                .or_filter(users::username.like(format!("%{}%", search_term)))
                .into_boxed();
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
            query.load::<(Uuid, String)>(conn).await.unwrap()
        }
    }
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
