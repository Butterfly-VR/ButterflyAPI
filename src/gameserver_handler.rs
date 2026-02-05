use crate::kube_resources::*;
use crate::{ApiError, AppState};
use axum::http::StatusCode;
use kube::Api;
use kube::api::PostParams;
use std::collections::BTreeMap;
use uuid::Uuid;

pub async fn allocate_gameserver(
    state: AppState,
    id: Uuid,
    instance_token: [u8; 64],
    world: Uuid,
    _dedicated_gameserver: bool,
) -> Result<(), ApiError> {
    let client = state.kube_client;

    let mut labels: BTreeMap<String, String> = BTreeMap::new();
    labels.insert("world".to_string(), world.to_string());
    labels.insert(
        "token".to_string(),
        serde_json::to_string(&instance_token.to_vec())?,
    );

    let gameserver_allocation = GameServerAllocation::new(
        &id.to_string(),
        GameServerAllocationSpec {
            selectors: vec![GameServerSelector {
                game_server_state: Some(GameServerState::Ready),
                ..Default::default()
            }],
            scheduling: Some(SchedulingStrategy::Distributed),
            metadata: Some(MetadataPatch {
                labels,
                ..Default::default()
            }),
            ..Default::default()
        },
    );

    Api::all(client)
        .create(&PostParams::default(), &gameserver_allocation)
        .await?;
    Ok(())
}

// todo: return error indicating that the gameserver is not ready
pub async fn get_connect_token(state: AppState, id: Uuid) -> Result<Vec<u8>, ApiError> {
    let client = state.kube_client;

    let gameserver: GameServerAllocation = Api::all(client).get(&id.to_string()).await?;

    let token: Vec<u8> = gameserver
        .spec
        .metadata
        .ok_or(ApiError::WithCode(StatusCode::INTERNAL_SERVER_ERROR))?
        .labels
        .get("token")
        .map(String::as_str)
        .map(serde_json::from_str)
        .ok_or(ApiError::WithCode(StatusCode::INTERNAL_SERVER_ERROR))??;

    return Ok(token);
}
