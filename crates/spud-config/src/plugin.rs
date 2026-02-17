use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use anyhow::{bail, Context, Result};
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};

/// Plugin manifest schema loaded from `plugin.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub runtime: PluginRuntime,
    pub compatibility: PluginCompatibility,
    pub permissions: PluginPermissions,
}

/// Runtime entrypoint metadata for plugin startup.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PluginRuntime {
    pub entrypoint: String,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
}

/// Compatibility constraints for host API negotiation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PluginCompatibility {
    pub host_api: String,
}

/// Permission allowlists for runtime host invocation checks.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PluginPermissions {
    #[serde(default)]
    pub commands: Vec<String>,
    #[serde(default)]
    pub event_tags: Vec<String>,
    #[serde(default)]
    pub subscriptions: Vec<String>,
}

impl PluginManifest {
    /// Parse and validate manifest TOML.
    pub fn from_toml_str(input: &str) -> Result<Self> {
        let manifest: Self =
            toml::from_str(input).context("failed to parse plugin manifest TOML")?;
        manifest.validate()?;
        Ok(manifest)
    }

    /// Load and validate a manifest from disk.
    pub fn from_path(path: &Path) -> Result<Self> {
        let raw = fs::read_to_string(path)
            .with_context(|| format!("failed to read plugin manifest at {}", path.display()))?;

        Self::from_toml_str(&raw)
            .with_context(|| format!("invalid plugin manifest at {}", path.display()))
    }

    /// Validate required fields and semantic constraints.
    pub fn validate(&self) -> Result<()> {
        validate_nonempty("id", &self.id)?;
        validate_nonempty("name", &self.name)?;
        validate_nonempty("version", &self.version)?;
        validate_nonempty("runtime.entrypoint", &self.runtime.entrypoint)?;
        validate_nonempty("compatibility.host_api", &self.compatibility.host_api)?;

        if let Some(command) = &self.runtime.command {
            validate_nonempty("runtime.command", command)?;
        }

        validate_runtime_args(&self.runtime.args)?;
        validate_allowlist("permissions.commands", &self.permissions.commands)?;
        validate_allowlist("permissions.event_tags", &self.permissions.event_tags)?;
        validate_allowlist("permissions.subscriptions", &self.permissions.subscriptions)?;

        Version::parse(&self.version)
            .with_context(|| format!("manifest version must be valid semver: {}", self.version))?;
        VersionReq::parse(&self.compatibility.host_api).with_context(|| {
            format!(
                "compatibility.host_api must be a valid semver requirement: {}",
                self.compatibility.host_api
            )
        })?;

        Ok(())
    }

    /// Ensure this plugin supports the provided host API version.
    pub fn supports_host_api(&self, host_api_version: &str) -> Result<()> {
        let requirement = VersionReq::parse(&self.compatibility.host_api).with_context(|| {
            format!(
                "compatibility.host_api must be a valid semver requirement: {}",
                self.compatibility.host_api
            )
        })?;
        let host = Version::parse(host_api_version).with_context(|| {
            format!("host API version must be valid semver: {host_api_version}")
        })?;

        if requirement.matches(&host) {
            Ok(())
        } else {
            bail!(
                "plugin requires host_api {} but host is {}",
                self.compatibility.host_api,
                host
            )
        }
    }
}

impl PluginPermissions {
    /// Return true when command invocation is allowlisted.
    pub fn allows_command(&self, command: &str) -> bool {
        self.commands.iter().any(|allowed| allowed == command)
    }

    /// Return true when publishing a custom event tag is allowlisted.
    pub fn allows_event_tag(&self, tag: &str) -> bool {
        self.event_tags.iter().any(|allowed| allowed == tag)
    }

    /// Return true when subscribing to a category is allowlisted.
    pub fn allows_subscription(&self, category: &str) -> bool {
        self.subscriptions.iter().any(|allowed| allowed == category)
    }
}

fn validate_nonempty(field: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        bail!("{field} must not be empty")
    }
    Ok(())
}

fn validate_runtime_args(args: &[String]) -> Result<()> {
    for arg in args {
        if arg.trim().is_empty() {
            bail!("runtime.args entries must not be empty");
        }
        if arg.trim() != arg {
            bail!(
                "runtime.args entry {:?} has leading/trailing whitespace",
                arg
            );
        }
    }

    Ok(())
}

fn validate_allowlist(field: &str, values: &[String]) -> Result<()> {
    let mut seen = BTreeSet::new();

    for value in values {
        if value.trim().is_empty() {
            bail!("{field} entries must not be empty");
        }
        if value.trim() != value {
            bail!("{field} entry {:?} has leading/trailing whitespace", value);
        }
        if !seen.insert(value.as_str()) {
            bail!("{field} contains duplicate entry {:?}", value);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_MANIFEST: &str = r#"
id = "spud.test"
name = "SPUD Test Plugin"
version = "0.1.0"

[runtime]
entrypoint = "dist/index.js"
command = "node"
args = ["--enable-source-maps"]

[compatibility]
host_api = "^1.0.0"

[permissions]
commands = ["help", "switch"]
event_tags = ["plugin.metrics"]
subscriptions = ["tick", "resize"]
"#;

    #[test]
    fn parses_valid_manifest() {
        let manifest = PluginManifest::from_toml_str(VALID_MANIFEST).unwrap();
        assert_eq!(manifest.id, "spud.test");
        assert!(manifest.permissions.allows_command("help"));
        assert!(manifest.permissions.allows_event_tag("plugin.metrics"));
        assert!(manifest.permissions.allows_subscription("tick"));
    }

    #[test]
    fn malformed_manifest_missing_required_section_is_rejected() {
        let raw = r#"
id = "spud.test"
name = "SPUD Test Plugin"
version = "0.1.0"

[runtime]
entrypoint = "dist/index.js"

[permissions]
commands = ["help"]
"#;

        let err = PluginManifest::from_toml_str(raw).unwrap_err().to_string();
        assert!(err.contains("failed to parse plugin manifest TOML"));
    }

    #[test]
    fn invalid_semver_is_rejected() {
        let raw = VALID_MANIFEST.replace("version = \"0.1.0\"", "version = \"not-semver\"");
        let err = PluginManifest::from_toml_str(&raw).unwrap_err().to_string();
        assert!(err.contains("manifest version must be valid semver"));
    }

    #[test]
    fn invalid_host_api_requirement_is_rejected() {
        let raw = VALID_MANIFEST.replace("host_api = \"^1.0.0\"", "host_api = \"what\"");
        let err = PluginManifest::from_toml_str(&raw).unwrap_err().to_string();
        assert!(err.contains("compatibility.host_api must be a valid semver requirement"));
    }

    #[test]
    fn duplicate_allowlist_entries_are_rejected() {
        let raw = VALID_MANIFEST.replace(
            "commands = [\"help\", \"switch\"]",
            "commands = [\"help\", \"help\"]",
        );
        let err = PluginManifest::from_toml_str(&raw).unwrap_err().to_string();
        assert!(err.contains("permissions.commands contains duplicate entry"));
    }

    #[test]
    fn compatibility_check_rejects_incompatible_host() {
        let manifest = PluginManifest::from_toml_str(VALID_MANIFEST).unwrap();
        let err = manifest.supports_host_api("2.0.0").unwrap_err().to_string();
        assert!(err.contains("plugin requires host_api"));
    }

    #[test]
    fn runtime_args_allow_duplicate_flags() {
        let raw = VALID_MANIFEST.replace(
            "args = [\"--enable-source-maps\"]",
            "args = [\"-v\", \"-v\"]",
        );
        let manifest = PluginManifest::from_toml_str(&raw).unwrap();
        assert_eq!(
            manifest.runtime.args,
            vec!["-v".to_string(), "-v".to_string()]
        );
    }
}
