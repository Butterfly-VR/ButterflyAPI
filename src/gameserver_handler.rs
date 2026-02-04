use crate::kube_resources::*;
use crate::{ApiError, AppState};
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

// todo: theres a race condition here if two clients connect to a server at the same time
// they get served the same connection token. probably need some kind of dirty flag
// so the second client knows to wait. one option would be a last_used_token column on
// the instance table
pub async fn get_connect_token(state: AppState, id: Uuid) -> Result<[u8; 1024], ApiError> {}
