use std::collections::BTreeSet;
use std::fmt;

use semver::{Version, VersionReq};
use spud_config::PluginManifest;

use crate::protocol::{
    error_code, EventCategory, InvokeCommandParams, JsonRpcError, PublishEventParams,
    HOST_API_VERSION,
};

/// Runtime permission policy built from a validated plugin manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionPolicy {
    host_api_requirement: String,
    commands: BTreeSet<String>,
    event_tags: BTreeSet<String>,
    subscriptions: BTreeSet<String>,
}

/// Permission and compatibility failures mapped to structured JSON-RPC errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthorizationError {
    InvalidHostApiRequirement(String),
    InvalidHostApiVersion(String),
    UnsupportedHostApi { required: String, host: String },
    UnauthorizedCommand(String),
    UnauthorizedEventTag(String),
    UnauthorizedSubscriptions(Vec<String>),
}

impl AuthorizationError {
    /// Map authorization failure to JSON-RPC host error code.
    pub fn code(&self) -> i32 {
        match self {
            Self::InvalidHostApiRequirement(_) | Self::InvalidHostApiVersion(_) => {
                error_code::INVALID_PARAMS
            }
            Self::UnsupportedHostApi { .. } => error_code::UNSUPPORTED_API_VERSION,
            Self::UnauthorizedCommand(_)
            | Self::UnauthorizedEventTag(_)
            | Self::UnauthorizedSubscriptions(_) => error_code::UNAUTHORIZED,
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

impl fmt::Display for AuthorizationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidHostApiRequirement(req) => {
                write!(
                    f,
                    "invalid manifest compatibility.host_api requirement: {req}"
                )
            }
            Self::InvalidHostApiVersion(version) => {
                write!(f, "invalid host API version: {version}")
            }
            Self::UnsupportedHostApi { required, host } => {
                write!(
                    f,
                    "plugin requires host_api {required}, but host API is {host}"
                )
            }
            Self::UnauthorizedCommand(command) => {
                write!(f, "command is not allowlisted: {command}")
            }
            Self::UnauthorizedEventTag(tag) => {
                write!(f, "event tag is not allowlisted: {tag}")
            }
            Self::UnauthorizedSubscriptions(categories) => write!(
                f,
                "subscription categories are not allowlisted: {}",
                categories.join(", ")
            ),
        }
    }
}

impl std::error::Error for AuthorizationError {}

impl PermissionPolicy {
    /// Build a policy snapshot from plugin manifest allowlists.
    pub fn from_manifest(manifest: &PluginManifest) -> Self {
        Self {
            host_api_requirement: manifest.compatibility.host_api.clone(),
            commands: manifest.permissions.commands.iter().cloned().collect(),
            event_tags: manifest.permissions.event_tags.iter().cloned().collect(),
            subscriptions: manifest.permissions.subscriptions.iter().cloned().collect(),
        }
    }

    /// Ensure plugin compatibility requirement matches host API version.
    pub fn ensure_host_compatibility(&self) -> std::result::Result<(), AuthorizationError> {
        let requirement = VersionReq::parse(&self.host_api_requirement).map_err(|_| {
            AuthorizationError::InvalidHostApiRequirement(self.host_api_requirement.clone())
        })?;
        let host = Version::parse(HOST_API_VERSION)
            .map_err(|_| AuthorizationError::InvalidHostApiVersion(HOST_API_VERSION.to_string()))?;

        if requirement.matches(&host) {
            Ok(())
        } else {
            Err(AuthorizationError::UnsupportedHostApi {
                required: self.host_api_requirement.clone(),
                host: host.to_string(),
            })
        }
    }

    /// Enforce command invocation allowlist.
    pub fn authorize_invoke_command(
        &self,
        params: &InvokeCommandParams,
    ) -> std::result::Result<(), AuthorizationError> {
        if self.commands.contains(&params.command) {
            Ok(())
        } else {
            Err(AuthorizationError::UnauthorizedCommand(
                params.command.clone(),
            ))
        }
    }

    /// Enforce event publish allowlist.
    pub fn authorize_publish_event(
        &self,
        params: &PublishEventParams,
    ) -> std::result::Result<(), AuthorizationError> {
        if self.event_tags.contains(&params.tag) {
            Ok(())
        } else {
            Err(AuthorizationError::UnauthorizedEventTag(params.tag.clone()))
        }
    }

    /// Enforce event subscription allowlist and return authorized categories.
    pub fn authorize_subscriptions(
        &self,
        categories: &[EventCategory],
    ) -> std::result::Result<Vec<EventCategory>, AuthorizationError> {
        let mut authorized = Vec::new();
        let mut unauthorized = Vec::new();
        let mut seen = BTreeSet::new();

        for category in categories {
            let name = event_category_name(*category);
            if !seen.insert(name) {
                continue;
            }

            if self.subscriptions.contains(name) {
                authorized.push(*category);
            } else {
                unauthorized.push(name.to_string());
            }
        }

        if unauthorized.is_empty() {
            Ok(authorized)
        } else {
            Err(AuthorizationError::UnauthorizedSubscriptions(unauthorized))
        }
    }
}

/// Validate manifest compatibility and return a policy ready for runtime checks.
pub fn policy_from_manifest(
    manifest: &PluginManifest,
) -> std::result::Result<PermissionPolicy, AuthorizationError> {
    let policy = PermissionPolicy::from_manifest(manifest);
    policy.ensure_host_compatibility()?;
    Ok(policy)
}

fn event_category_name(category: EventCategory) -> &'static str {
    match category {
        EventCategory::Tick => "tick",
        EventCategory::Resize => "resize",
        EventCategory::ModuleLifecycle => "module_lifecycle",
        EventCategory::Telemetry => "telemetry",
        EventCategory::Custom => "custom",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::error_code;

    fn manifest_with_permissions(
        host_api: &str,
        commands: &[&str],
        event_tags: &[&str],
        subscriptions: &[&str],
    ) -> PluginManifest {
        let commands = commands
            .iter()
            .map(|command| format!("\"{command}\""))
            .collect::<Vec<_>>()
            .join(", ");
        let event_tags = event_tags
            .iter()
            .map(|tag| format!("\"{tag}\""))
            .collect::<Vec<_>>()
            .join(", ");
        let subscriptions = subscriptions
            .iter()
            .map(|subscription| format!("\"{subscription}\""))
            .collect::<Vec<_>>()
            .join(", ");

        let raw = format!(
            r#"
id = "spud.test"
name = "SPUD Test Plugin"
version = "0.1.0"

[runtime]
entrypoint = "dist/index.js"

[compatibility]
host_api = "{host_api}"

[permissions]
commands = [{commands}]
event_tags = [{event_tags}]
subscriptions = [{subscriptions}]
"#
        );

        PluginManifest::from_toml_str(&raw).unwrap()
    }

    #[test]
    fn authorize_invoke_command_accepts_allowlisted_command() {
        let manifest = manifest_with_permissions("^1.0.0", &["help"], &[], &[]);
        let policy = policy_from_manifest(&manifest).unwrap();

        let params = InvokeCommandParams {
            command: "help".to_string(),
            args: vec![],
        };

        assert!(policy.authorize_invoke_command(&params).is_ok());
    }

    #[test]
    fn authorize_invoke_command_denies_unallowlisted_command() {
        let manifest = manifest_with_permissions("^1.0.0", &["help"], &[], &[]);
        let policy = policy_from_manifest(&manifest).unwrap();

        let params = InvokeCommandParams {
            command: "quit".to_string(),
            args: vec![],
        };

        let err = policy.authorize_invoke_command(&params).unwrap_err();
        assert!(matches!(err, AuthorizationError::UnauthorizedCommand(_)));
        assert_eq!(err.code(), error_code::UNAUTHORIZED);
    }

    #[test]
    fn authorize_publish_event_denies_unallowlisted_tag() {
        let manifest = manifest_with_permissions("^1.0.0", &[], &["plugin.metrics"], &[]);
        let policy = policy_from_manifest(&manifest).unwrap();

        let params = PublishEventParams {
            tag: "plugin.debug".to_string(),
            payload: "{}".to_string(),
        };

        let err = policy.authorize_publish_event(&params).unwrap_err();
        assert!(matches!(err, AuthorizationError::UnauthorizedEventTag(_)));
        assert_eq!(err.code(), error_code::UNAUTHORIZED);
    }

    #[test]
    fn authorize_subscriptions_denies_over_privileged_categories() {
        let manifest = manifest_with_permissions("^1.0.0", &[], &[], &["tick", "resize"]);
        let policy = policy_from_manifest(&manifest).unwrap();

        let err = policy
            .authorize_subscriptions(&[EventCategory::Tick, EventCategory::Custom])
            .unwrap_err();

        assert_eq!(
            err,
            AuthorizationError::UnauthorizedSubscriptions(vec!["custom".to_string()])
        );
        assert_eq!(err.code(), error_code::UNAUTHORIZED);
    }

    #[test]
    fn policy_rejects_incompatible_host_api() {
        let manifest = manifest_with_permissions("^2.0.0", &[], &[], &[]);
        let err = policy_from_manifest(&manifest).unwrap_err();

        assert!(matches!(err, AuthorizationError::UnsupportedHostApi { .. }));
        assert_eq!(err.code(), error_code::UNSUPPORTED_API_VERSION);
    }

    #[test]
    fn reload_revalidation_applies_updated_permissions() {
        let initial = manifest_with_permissions("^1.0.0", &["help"], &[], &[]);
        let initial_policy = policy_from_manifest(&initial).unwrap();
        let command = InvokeCommandParams {
            command: "help".to_string(),
            args: vec![],
        };
        assert!(initial_policy.authorize_invoke_command(&command).is_ok());

        let reloaded = manifest_with_permissions("^1.0.0", &["switch"], &[], &[]);
        let reloaded_policy = policy_from_manifest(&reloaded).unwrap();
        let err = reloaded_policy
            .authorize_invoke_command(&command)
            .unwrap_err();
        assert!(matches!(err, AuthorizationError::UnauthorizedCommand(_)));
    }
}
