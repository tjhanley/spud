use std::collections::BTreeSet;
use std::fmt;

use anyhow::{anyhow, Context, Result};
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// JSON-RPC protocol version used for plugin transport.
pub const JSONRPC_VERSION: &str = "2.0";
/// OpenRPC spec version for the embedded contract document.
pub const OPENRPC_VERSION: &str = "1.3.2";
/// Host API version negotiated during `spud.handshake`.
pub const HOST_API_VERSION: &str = "1.0.0";

/// Embedded OpenRPC contract document (source of truth for method schema).
pub const OPENRPC_SPEC_JSON: &str = include_str!("../openrpc/spud-plugin-host-v1.openrpc.json");

/// Methods that must exist in the OpenRPC contract.
pub const REQUIRED_METHODS: [&str; 6] = [
    "spud.handshake",
    "spud.state.get_snapshot",
    "spud.events.subscribe",
    "spud.events.unsubscribe",
    "spud.host.invoke_command",
    "spud.host.publish_event",
];

/// Host JSON-RPC error codes.
pub mod error_code {
    /// JSON-RPC standard invalid params error.
    pub const INVALID_PARAMS: i32 = -32602;
    /// API version is unsupported by the host.
    pub const UNSUPPORTED_API_VERSION: i32 = -32001;
    /// Plugin attempted an operation that is not authorized.
    pub const UNAUTHORIZED: i32 = -32002;
    /// Plugin host transport/runtime is unavailable.
    pub const PLUGIN_UNAVAILABLE: i32 = -32003;
}

/// JSON-RPC request/response ID type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RequestId {
    String(String),
    Number(i64),
    Null,
}

/// JSON-RPC error payload emitted by the host.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// Handshake-specific negotiation failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandshakeError {
    InvalidVersionRequirement(String),
    UnsupportedApiVersion { host: String, supported: String },
    UnsupportedRequestedCapabilities(Vec<String>),
    HostApiVersionInvalid(String),
    HostCapabilitiesUnavailable(String),
}

impl HandshakeError {
    /// Map handshake failure to JSON-RPC host error code.
    pub fn code(&self) -> i32 {
        match self {
            Self::InvalidVersionRequirement(_) => error_code::INVALID_PARAMS,
            Self::UnsupportedApiVersion { .. } => error_code::UNSUPPORTED_API_VERSION,
            Self::UnsupportedRequestedCapabilities(_) => error_code::INVALID_PARAMS,
            Self::HostApiVersionInvalid(_) | Self::HostCapabilitiesUnavailable(_) => {
                error_code::PLUGIN_UNAVAILABLE
            }
        }
    }

    /// Convert to a JSON-RPC error payload.
    pub fn to_jsonrpc_error(&self) -> JsonRpcError {
        JsonRpcError {
            code: self.code(),
            message: self.to_string(),
            data: None,
        }
    }
}

impl fmt::Display for HandshakeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidVersionRequirement(req) => {
                write!(f, "invalid supported_api_versions requirement: {req}")
            }
            Self::UnsupportedApiVersion { host, supported } => write!(
                f,
                "host API version {host} is not compatible with plugin requirement {supported}"
            ),
            Self::UnsupportedRequestedCapabilities(requested) => write!(
                f,
                "requested_capabilities contains no supported entries: {}",
                requested.join(", ")
            ),
            Self::HostApiVersionInvalid(version) => {
                write!(
                    f,
                    "host API version constant is not valid semver: {version}"
                )
            }
            Self::HostCapabilitiesUnavailable(err) => {
                write!(f, "host capabilities unavailable: {err}")
            }
        }
    }
}

impl std::error::Error for HandshakeError {}

/// Plugin event categories exposed by host subscription methods.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventCategory {
    Tick,
    Resize,
    ModuleLifecycle,
    Telemetry,
    Custom,
}

impl EventCategory {
    /// Every event category the host supports.
    pub const ALL: [EventCategory; 5] = [
        EventCategory::Tick,
        EventCategory::Resize,
        EventCategory::ModuleLifecycle,
        EventCategory::Telemetry,
        EventCategory::Custom,
    ];

    fn as_str(self) -> &'static str {
        match self {
            EventCategory::Tick => "tick",
            EventCategory::Resize => "resize",
            EventCategory::ModuleLifecycle => "module_lifecycle",
            EventCategory::Telemetry => "telemetry",
            EventCategory::Custom => "custom",
        }
    }
}

/// Parameters for `spud.handshake`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HandshakeParams {
    pub plugin_id: String,
    pub plugin_version: String,
    pub supported_api_versions: String,
    #[serde(default)]
    pub requested_capabilities: Vec<String>,
}

/// Result payload for `spud.handshake`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HandshakeResult {
    pub selected_api_version: String,
    pub host_capabilities: HostCapabilities,
}

/// Host capabilities advertised in handshake.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostCapabilities {
    pub methods: Vec<String>,
    pub event_categories: Vec<EventCategory>,
}

/// Parameters for `spud.state.get_snapshot`.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetSnapshotParams {}

/// Active module information exposed in state snapshots.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActiveModule {
    pub id: String,
    pub title: String,
}

/// A single telemetry item in state snapshots.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TelemetryDatum {
    pub source: String,
    pub key: String,
    pub value: Value,
}

/// Read-only host state snapshot exposed to plugins.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StateSnapshot {
    pub active_module: Option<ActiveModule>,
    pub status_line: String,
    pub uptime_seconds: u64,
    pub tps: f64,
    pub telemetry: Vec<TelemetryDatum>,
}

/// Parameters for subscription methods.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubscribeParams {
    pub categories: Vec<EventCategory>,
}

/// Result payload for subscription methods.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubscriptionResult {
    pub subscribed: Vec<EventCategory>,
}

/// Parameters for `spud.host.invoke_command`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvokeCommandParams {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
}

/// Result payload for `spud.host.invoke_command`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvokeCommandResult {
    pub lines: Vec<String>,
}

/// Parameters for `spud.host.publish_event`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublishEventParams {
    pub tag: String,
    pub payload: String,
}

/// Result payload for `spud.host.publish_event`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublishEventResult {
    pub accepted: bool,
}

#[derive(Debug, Deserialize)]
struct OpenRpcDocument {
    openrpc: String,
    methods: Vec<OpenRpcMethod>,
}

#[derive(Debug, Deserialize)]
struct OpenRpcMethod {
    name: String,
}

/// Parse and validate the embedded OpenRPC document.
pub fn validate_openrpc_spec() -> Result<()> {
    let doc: OpenRpcDocument = serde_json::from_str(OPENRPC_SPEC_JSON)
        .context("failed to parse embedded OpenRPC document")?;

    if doc.openrpc != OPENRPC_VERSION {
        return Err(anyhow!(
            "embedded OpenRPC version {} does not match expected {}",
            doc.openrpc,
            OPENRPC_VERSION
        ));
    }

    let methods: BTreeSet<_> = doc.methods.iter().map(|m| m.name.as_str()).collect();
    let required: BTreeSet<_> = REQUIRED_METHODS.into_iter().collect();

    if methods != required {
        return Err(anyhow!(
            "OpenRPC method set mismatch. expected={:?} actual={:?}",
            required,
            methods
        ));
    }

    Ok(())
}

/// Read method names from embedded OpenRPC spec.
pub fn openrpc_method_names() -> Result<Vec<String>> {
    let doc: OpenRpcDocument = serde_json::from_str(OPENRPC_SPEC_JSON)
        .context("failed to parse embedded OpenRPC document")?;
    Ok(doc.methods.into_iter().map(|m| m.name).collect())
}

/// Host capability snapshot derived from OpenRPC contract and event categories.
pub fn host_capabilities() -> Result<HostCapabilities> {
    Ok(HostCapabilities {
        methods: openrpc_method_names()?,
        event_categories: EventCategory::ALL.to_vec(),
    })
}

/// Negotiate host API version against plugin semver requirement string.
pub fn negotiate_api_version(
    supported_api_versions: &str,
) -> std::result::Result<String, HandshakeError> {
    let supported = VersionReq::parse(supported_api_versions).map_err(|_| {
        HandshakeError::InvalidVersionRequirement(supported_api_versions.to_string())
    })?;

    let host = Version::parse(HOST_API_VERSION)
        .map_err(|_| HandshakeError::HostApiVersionInvalid(HOST_API_VERSION.to_string()))?;

    if supported.matches(&host) {
        Ok(host.to_string())
    } else {
        Err(HandshakeError::UnsupportedApiVersion {
            host: host.to_string(),
            supported: supported_api_versions.to_string(),
        })
    }
}

/// Build a handshake result from plugin params using version negotiation.
pub fn build_handshake_result(
    params: &HandshakeParams,
) -> std::result::Result<HandshakeResult, HandshakeError> {
    let selected = negotiate_api_version(&params.supported_api_versions)?;
    let all_capabilities = host_capabilities()
        .map_err(|err| HandshakeError::HostCapabilitiesUnavailable(err.to_string()))?;
    let capabilities = filter_host_capabilities(all_capabilities, &params.requested_capabilities)?;

    Ok(HandshakeResult {
        selected_api_version: selected,
        host_capabilities: capabilities,
    })
}

fn filter_host_capabilities(
    all: HostCapabilities,
    requested: &[String],
) -> std::result::Result<HostCapabilities, HandshakeError> {
    if requested.is_empty() {
        return Ok(all);
    }

    let requested: BTreeSet<_> = requested.iter().map(String::as_str).collect();

    let methods = all
        .methods
        .into_iter()
        .filter(|method| requested.contains(method.as_str()))
        .collect::<Vec<_>>();

    let event_categories = all
        .event_categories
        .into_iter()
        .filter(|category| requested.contains(category.as_str()))
        .collect::<Vec<_>>();

    if methods.is_empty() && event_categories.is_empty() {
        return Err(HandshakeError::UnsupportedRequestedCapabilities(
            requested.into_iter().map(str::to_string).collect(),
        ));
    }

    Ok(HostCapabilities {
        methods,
        event_categories,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openrpc_spec_is_valid_and_complete() {
        validate_openrpc_spec().unwrap();
    }

    #[test]
    fn host_capabilities_read_methods_from_spec() {
        let caps = host_capabilities().unwrap();
        assert_eq!(caps.methods.len(), REQUIRED_METHODS.len());
        for required in REQUIRED_METHODS {
            assert!(caps.methods.iter().any(|m| m == required));
        }
        assert_eq!(caps.event_categories, EventCategory::ALL.to_vec());
    }

    #[test]
    fn negotiate_api_version_accepts_compatible_range() {
        let selected = negotiate_api_version("^1.0").unwrap();
        assert_eq!(selected, HOST_API_VERSION);
    }

    #[test]
    fn negotiate_api_version_rejects_incompatible_range() {
        let err = negotiate_api_version("^2.0").unwrap_err();
        assert!(matches!(err, HandshakeError::UnsupportedApiVersion { .. }));
        assert_eq!(err.code(), error_code::UNSUPPORTED_API_VERSION);
    }

    #[test]
    fn negotiate_api_version_rejects_invalid_range() {
        let err = negotiate_api_version("not-a-version").unwrap_err();
        assert!(matches!(err, HandshakeError::InvalidVersionRequirement(_)));
        assert_eq!(err.code(), error_code::INVALID_PARAMS);
    }

    #[test]
    fn handshake_result_includes_capabilities_and_selected_version() {
        let params = HandshakeParams {
            plugin_id: "test.plugin".to_string(),
            plugin_version: "0.1.0".to_string(),
            supported_api_versions: "^1.0".to_string(),
            requested_capabilities: vec![],
        };

        let result = build_handshake_result(&params).unwrap();
        assert_eq!(result.selected_api_version, HOST_API_VERSION);
        assert!(result
            .host_capabilities
            .methods
            .iter()
            .any(|m| m == "spud.handshake"));
    }

    #[test]
    fn handshake_requested_capabilities_filters_results() {
        let params = HandshakeParams {
            plugin_id: "test.plugin".to_string(),
            plugin_version: "0.1.0".to_string(),
            supported_api_versions: "^1.0".to_string(),
            requested_capabilities: vec![
                "spud.state.get_snapshot".to_string(),
                "telemetry".to_string(),
            ],
        };

        let result = build_handshake_result(&params).unwrap();
        assert_eq!(
            result.host_capabilities.methods,
            vec!["spud.state.get_snapshot".to_string()]
        );
        assert_eq!(
            result.host_capabilities.event_categories,
            vec![EventCategory::Telemetry]
        );
    }

    #[test]
    fn handshake_requested_capabilities_rejects_unsupported_only() {
        let params = HandshakeParams {
            plugin_id: "test.plugin".to_string(),
            plugin_version: "0.1.0".to_string(),
            supported_api_versions: "^1.0".to_string(),
            requested_capabilities: vec!["not.supported".to_string()],
        };

        let err = build_handshake_result(&params).unwrap_err();
        assert!(matches!(
            err,
            HandshakeError::UnsupportedRequestedCapabilities(_)
        ));
        assert_eq!(err.code(), error_code::INVALID_PARAMS);
    }
}
