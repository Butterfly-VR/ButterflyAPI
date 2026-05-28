use crate::kube_resources::{
    GameServerAllocation, GameServerAllocationSpec, GameServerSelector, GameServerState,
    MetadataPatch, SchedulingStrategy,
};
use crate::{ApiError, AppState};
use axum::http::StatusCode;
use kube::Api;
use kube::api::PostParams;
use std::collections::BTreeMap;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use uuid::Uuid;

pub async fn allocate_gameserver(
    state: Arc<AppState>,
    id: Uuid,
    instance_token: [u8; 64],
    world: Uuid,
    _dedicated_gameserver: bool,
) -> Result<SocketAddr, ApiError> {
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

    let allocation = Api::namespaced(client.clone(), "default")
        .create(&PostParams::default(), &gameserver_allocation)
        .await?;

    let Some((ip, port)) = allocation
        .status
        .map(|x| (x.address, x.ports.get(0).map(|x| x.port)))
    else {
        return Err(ApiError::WithCode(StatusCode::INTERNAL_SERVER_ERROR));
    };

    let ip = ip
        .and_then(|x| x.parse::<Ipv4Addr>().ok())
        .ok_or(ApiError::WithCode(StatusCode::INTERNAL_SERVER_ERROR))?;

    let port = port
        .and_then(|x| x.try_into().ok())
        .ok_or(ApiError::WithCode(StatusCode::INTERNAL_SERVER_ERROR))?;

    Ok(SocketAddr::new(ip.into(), port))
}
