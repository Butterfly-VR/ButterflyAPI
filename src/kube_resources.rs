use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Used to allocate a `GameServer` from a pool of available `GameServers`.
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

/// Selector for filtering `GameServers`.
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

/// A selector that contains values, a key, and an operator.
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
    #[allow(clippy::struct_field_names)]
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
