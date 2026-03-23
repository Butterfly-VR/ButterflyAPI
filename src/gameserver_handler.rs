use crate::kube_resources::{
    GameServerAllocation, GameServerAllocationSpec, GameServerSelector, GameServerState,
    MetadataPatch, SchedulingStrategy,
};
use crate::{ApiError, AppState};
use kube::Api;
use kube::api::PostParams;
use std::collections::BTreeMap;
use std::sync::Arc;
use uuid::Uuid;

pub async fn allocate_gameserver(
    state: Arc<AppState>,
    id: Uuid,
    instance_token: [u8; 64],
    world: Uuid,
    _dedicated_gameserver: bool,
) -> Result<(), ApiError> {
    let client = state.kube_client.clone();

    let mut labels: BTreeMap<String, String> = BTreeMap::new();
    labels.insert("world".to_string(), world.to_string());

    let mut annotations: BTreeMap<String, String> = BTreeMap::new();
    annotations.insert("token".to_string(), hex::encode(instance_token));

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
                annotations,
            }),
            ..Default::default()
        },
    );

    Api::namespaced(client, "default")
        .create(&PostParams::default(), &gameserver_allocation)
        .await?;
    Ok(())
}
