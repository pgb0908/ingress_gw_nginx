use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct GatewayDocument {
    pub metadata: Metadata,
    pub spec: GatewaySpec,
}

#[derive(Debug, Deserialize)]
pub struct Metadata {
    pub name: String,
}

#[derive(Debug, Default, Deserialize)]
pub struct GatewaySpec {
    #[serde(default)]
    pub logging: GatewayLogging,
    #[serde(default)]
    pub tracing: GatewayTracing,
    #[serde(default)]
    pub metrics: GatewayMetrics,
}

#[derive(Debug, Default, Deserialize)]
pub struct GatewayLogging {
    #[serde(rename = "accessLog", default)]
    pub access_log: GatewayAccessLog,
}

#[derive(Debug, Deserialize)]
pub struct GatewayAccessLog {
    #[serde(default = "default_true")]
    pub enabled: bool,
}

impl Default for GatewayAccessLog {
    fn default() -> Self {
        Self { enabled: true }
    }
}

#[derive(Debug, Default, Deserialize)]
pub struct GatewayTracing {
    #[serde(default)]
    pub enabled: bool,
}

#[derive(Debug, Deserialize)]
pub struct GatewayMetrics {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_metrics_path")]
    pub path: String,
    #[serde(default = "default_metrics_port")]
    pub port: u16,
}

impl Default for GatewayMetrics {
    fn default() -> Self {
        Self {
            enabled: true,
            path: default_metrics_path(),
            port: default_metrics_port(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ListenerDocument {
    pub metadata: Metadata,
    pub spec: ListenerSpec,
}

#[derive(Debug, Deserialize)]
pub struct ListenerSpec {
    pub protocol: String,
    pub port: u16,
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(rename = "allowedHostnames", default)]
    pub allowed_hostnames: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct RouterDocument {
    pub metadata: Metadata,
    pub spec: RouterSpec,
}

#[derive(Debug, Deserialize)]
pub struct RouterSpec {
    #[serde(rename = "targetRef")]
    pub target_ref: TargetRef,
    pub rules: Vec<RouterRule>,
    pub config: RouterConfig,
}

#[derive(Debug, Deserialize)]
pub struct TargetRef {
    pub kind: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct RouterRule {
    pub path: String,
    #[serde(default)]
    pub methods: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct RouterConfig {
    pub destinations: Vec<DestinationConfig>,
}

#[derive(Debug, Deserialize)]
pub struct DestinationConfig {
    #[serde(rename = "destinationRef")]
    pub destination_ref: TargetRef,
    #[serde(default = "default_weight")]
    pub weight: u16,
}

#[derive(Debug, Deserialize)]
pub struct ServiceDocument {
    pub metadata: Metadata,
    pub spec: ServiceSpec,
}

#[derive(Debug, Deserialize)]
pub struct ServiceSpec {
    #[serde(default = "default_protocol")]
    pub protocol: String,
    #[serde(rename = "loadBalancing")]
    pub load_balancing: LoadBalancing,
}

#[derive(Debug, Deserialize)]
pub struct LoadBalancing {
    pub targets: Vec<UpstreamTarget>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct UpstreamTarget {
    pub host: String,
    pub port: u16,
    #[serde(default = "default_weight")]
    pub weight: u16,
}

#[derive(Debug, Deserialize)]
pub struct PolicyDocument {
    pub metadata: Metadata,
    pub spec: PolicySpec,
}

#[derive(Debug, Deserialize)]
pub struct PolicySpec {
    #[serde(rename = "targetRef")]
    pub target_ref: TargetRef,
    #[serde(default = "default_policy_order")]
    pub order: u16,
    #[serde(default)]
    pub config: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct RevisionManifest {
    pub revision: String,
    #[serde(rename = "created_at")]
    pub created_at: String,
    #[serde(rename = "runtime_compat")]
    pub runtime_compat: String,
    #[serde(default)]
    pub plugins: Vec<PluginManifest>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    #[serde(rename = "wasm_path")]
    pub wasm_path: String,
    #[serde(default)]
    pub sha256: String,
    #[serde(rename = "failure_mode", default = "default_failure_mode")]
    pub failure_mode: String,
    #[serde(default)]
    pub hooks: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct PluginChainDocument {
    #[serde(default)]
    pub plugins: Vec<String>,
}

#[derive(Debug)]
pub struct RevisionBundle {
    pub root: PathBuf,
    pub manifest: RevisionManifest,
    pub gateway: GatewayDocument,
    pub listener: ListenerDocument,
    pub routers: Vec<RouterDocument>,
    pub services: HashMap<String, ServiceDocument>,
    pub policies: Vec<PolicyEntry>,
    pub plugin_chain: Vec<String>,
}

#[derive(Debug)]
pub struct PolicyEntry {
    pub document: PolicyDocument,
    pub source_file: PathBuf,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct RuntimeState {
    pub current_revision: Option<String>,
    pub current_revision_path: Option<String>,
    pub last_validation: Option<ValidationSnapshot>,
    pub last_reload_status: Option<ReloadStatus>,
    #[serde(default)]
    pub metrics: MetricsState,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ValidationSnapshot {
    pub revision: String,
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReloadStatus {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct MetricsState {
    pub gateway_reload_total: u64,
    pub gateway_reload_failures_total: u64,
    pub gateway_requests_total: u64,
    pub gateway_request_duration_ms: u64,
    pub gateway_plugin_executions_total: u64,
    pub gateway_plugin_failures_total: u64,
    pub gateway_policy_denied_total: u64,
    pub gateway_rate_limit_denied_total: u64,
}

#[derive(Debug, Serialize)]
pub struct ValidationResult {
    pub revision: String,
    pub valid: bool,
    pub rendered_conf: Option<String>,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct LoadResult {
    pub revision: Option<String>,
    pub status: String,
    pub message: String,
    pub validation: Option<ValidationResult>,
}

#[derive(Debug, Serialize)]
pub struct DeployResult {
    pub kind: String,
    pub name: String,
    /// "applied" | "staged" | "failed"
    pub status: String,
    pub message: String,
    pub validation: Option<ValidationResult>,
}

pub fn default_true() -> bool {
    true
}

fn default_weight() -> u16 {
    100
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_protocol() -> String {
    "HTTP".to_string()
}

fn default_failure_mode() -> String {
    "fail-open".to_string()
}

fn default_policy_order() -> u16 {
    100
}

fn default_metrics_path() -> String {
    "/metrics".to_string()
}

fn default_metrics_port() -> u16 {
    19090
}

