use k8s_openapi::api::core::v1::PodTemplateSpec;
use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// GameServerAllocation is used to allocate a GameServer from a pool of available GameServers.
#[derive(CustomResource, Clone, Debug, Default, Deserialize, Serialize, JsonSchema)]
#[kube(
    group = "allocation.agones.dev",
    version = "v1",
    kind = "GameServerAllocation",
    namespaced
)]
#[serde(rename_all = "camelCase")]
pub struct GameServerAllocationSpec {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub selectors: Vec<GameServerSelector>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scheduling: Option<SchedulingStrategy>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<MetadataPatch>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub priorities: Vec<Priority>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub counters: BTreeMap<String, CounterAction>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub lists: BTreeMap<String, ListAction>,
}

/// gameserver definition for retriving metadata after allocation
#[derive(CustomResource, Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[kube(group = "agones.dev", version = "v1", kind = "GameServer", namespaced)]
#[serde(rename_all = "camelCase")]
pub struct GameServerSpec {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub container: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ports: Vec<GameServerPort>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub health: Option<Health>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sdk_server: Option<SdkServer>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub counters: BTreeMap<String, Counter>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub lists: BTreeMap<String, GameServerList>,
    pub template: PodTemplateSpec,
}

/// Selector for filtering GameServers.
#[derive(Clone, Debug, Default, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GameServerSelector {
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub match_labels: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub match_expressions: Vec<LabelSelectorRequirement>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub game_server_state: Option<GameServerState>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub counters: BTreeMap<String, CounterSelector>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub lists: BTreeMap<String, ListSelector>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub players: Option<PlayerSelector>,
}

/// A label selector requirement is a selector that contains values, a key, and an operator.
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LabelSelectorRequirement {
    pub key: String,
    pub operator: LabelSelectorOperator,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub values: Vec<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, JsonSchema)]
pub enum LabelSelectorOperator {
    In,
    NotIn,
    Exists,
    DoesNotExist,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, JsonSchema)]
pub enum GameServerState {
    Ready,
    Allocated,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CounterSelector {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_count: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_count: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_available: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_available: Option<i64>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListSelector {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contains_value: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_available: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_available: Option<i64>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PlayerSelector {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_available: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_available: Option<i64>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, JsonSchema)]
pub enum SchedulingStrategy {
    Packed,
    Distributed,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MetadataPatch {
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub labels: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub annotations: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Priority {
    #[serde(rename = "type")]
    pub priority_type: PriorityType,
    pub key: String,
    pub order: PriorityOrder,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, JsonSchema)]
pub enum PriorityType {
    Counter,
    List,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, JsonSchema)]
pub enum PriorityOrder {
    Ascending,
    Descending,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CounterAction {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<CounterActionType>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capacity: Option<i64>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, JsonSchema)]
pub enum CounterActionType {
    Increment,
    Decrement,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListAction {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub add_values: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capacity: Option<i64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub delete_values: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GameServerPort {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub range: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port_policy: Option<PortPolicy>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub container: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub container_port: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_port: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protocol: Option<PortProtocol>,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
pub enum PortPolicy {
    Dynamic,
    Static,
    Passthrough,
    None,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
pub enum PortProtocol {
    UDP,
    TCP,
    TCPUDP,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Health {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub initial_delay_seconds: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub period_seconds: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure_threshold: Option<i32>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SdkServer {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub log_level: Option<SdkServerLogLevel>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grpc_port: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub http_port: Option<i32>,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
pub enum SdkServerLogLevel {
    Info,
    Debug,
    Error,
    Trace,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Counter {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capacity: Option<i64>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GameServerList {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capacity: Option<i64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub values: Vec<String>,
}
