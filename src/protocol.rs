use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const PROTOCOL_VERSION: &str = "1.0";
pub const HOST_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Serialize, Deserialize)]
pub struct RequestEnvelope {
    pub id: Value,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

#[derive(Debug, Serialize)]
pub struct ResponseEnvelope<T>
where
    T: Serialize,
{
    pub id: Value,
    pub result: T,
}

#[derive(Debug, Serialize)]
pub struct ErrorEnvelope {
    pub id: Value,
    pub error: ErrorPayload,
}

#[derive(Debug, Serialize)]
pub struct ErrorPayload {
    pub code: ErrorCode,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    InvalidRequest,
    MethodNotFound,
    ExtensionNotPaired,
    PairingRejected,
    PairingCancelled,
    ProviderUnknown,
    ProviderNotAllowed,
    ProviderNotConfigured,
    SessionLimitReached,
    NotSupported,
    HostNotFound,
    ManifestInvalid,
    OriginNotAllowed,
    InternalError,
}

#[derive(Debug, Deserialize)]
pub struct HelloParams {
    pub protocol_version: String,
    pub extension_name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct HelloResult {
    pub protocol_version: &'static str,
    pub host_version: &'static str,
    pub extension_id_seen: String,
    pub pairing_status: PairingStatus,
    pub supported_methods: Vec<&'static str>,
    pub capabilities: Capabilities,
}

#[derive(Debug, Deserialize)]
pub struct PairParams {
    pub extension_name: String,
    #[serde(default)]
    pub requested_providers: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct PairResult {
    pub pairing_status: PairingStatus,
    pub allowed_providers: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct StatusResult {
    pub host_version: &'static str,
    pub uptime_seconds: u64,
    pub pairing_status: PairingStatus,
    pub active_sessions: usize,
    pub providers: Vec<ProviderStatus>,
    pub admin_ui_url: String,
    pub wrapper_path: String,
    pub wrapper_present: bool,
}

#[derive(Debug, Serialize)]
pub struct ProviderStatus {
    pub id: String,
    pub configured: bool,
}

#[derive(Debug, Serialize)]
pub struct OkResult {
    pub ok: bool,
}

#[derive(Debug, Serialize)]
pub struct OpenSettingsResult {
    pub ok: bool,
    pub url: String,
}

#[derive(Debug, Serialize)]
pub struct ProviderListResult {
    pub providers: Vec<ProviderInfo>,
}

#[derive(Debug, Serialize)]
pub struct ProviderInfo {
    pub id: String,
    pub display_name: String,
    pub configured: bool,
}

#[derive(Debug, Deserialize)]
pub struct VaultSetSecretParams {
    pub provider: String,
    pub secret: String,
}

#[derive(Debug, Deserialize)]
pub struct VaultDeleteSecretParams {
    pub provider: String,
}

#[derive(Debug, Deserialize)]
pub struct OpenSessionParams {
    pub provider_id: String,
    pub operation: String,
}

#[derive(Debug, Serialize)]
pub struct OpenSessionResult {
    pub base_url: String,
    pub session_id: String,
    pub token: String,
    pub expires_at: DateTime<Utc>,
    pub provider_id: String,
    pub allowed_operation: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PairingStatus {
    Paired,
    Unpaired,
}

#[derive(Debug, Serialize)]
pub struct Capabilities {
    pub http_data_plane: bool,
    pub responses_api: bool,
}

pub fn supported_methods() -> Vec<&'static str> {
    vec![
        "bridge.hello",
        "bridge.pair",
        "bridge.status",
        "bridge.open_settings",
        "vault.list_providers",
        "vault.set_secret",
        "vault.delete_secret",
        "llm.open_session",
    ]
}
