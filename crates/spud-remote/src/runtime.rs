use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use spud_config::PluginManifest;

use crate::permissions::{policy_from_manifest, PermissionPolicy};
use crate::protocol::{
    build_handshake_result, error_code, EventCategory, GetSnapshotParams, HandshakeParams,
    HandshakeResult, InvokeCommandParams, InvokeCommandResult, JsonRpcError, PublishEventParams,
    PublishEventResult, RequestId, StateSnapshot, SubscribeParams, SubscriptionResult,
    JSONRPC_VERSION,
};

const HANDSHAKE_METHOD: &str = "spud.handshake";
const GET_SNAPSHOT_METHOD: &str = "spud.state.get_snapshot";
const SUBSCRIBE_METHOD: &str = "spud.events.subscribe";
const UNSUBSCRIBE_METHOD: &str = "spud.events.unsubscribe";
const INVOKE_COMMAND_METHOD: &str = "spud.host.invoke_command";
const PUBLISH_EVENT_METHOD: &str = "spud.host.publish_event";
const EVENT_NOTIFICATION_METHOD: &str = "spud.events.emit";

/// A plugin manifest discovered on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredPlugin {
    pub manifest_path: PathBuf,
    pub manifest: PluginManifest,
}

/// Host operations exposed to the runtime bridge.
pub trait HostBridge {
    /// Return a read-only host state snapshot.
    fn state_snapshot(&mut self) -> Result<StateSnapshot>;

    /// Execute a host command invocation requested by a plugin.
    fn invoke_command(&mut self, params: InvokeCommandParams) -> Result<InvokeCommandResult>;

    /// Publish a custom event requested by a plugin.
    fn publish_event(&mut self, params: PublishEventParams) -> Result<PublishEventResult>;
}

/// Outcome of handling one inbound plugin request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandledRequest {
    pub plugin_id: String,
    pub method: String,
    pub responded_with_error: bool,
}

/// Runtime manager failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeError {
    Discovery(String),
    Spawn(String),
    UnknownPlugin(String),
    AlreadyRunning(String),
    NotRunning(String),
    Timeout {
        plugin_id: String,
        timeout_ms: u64,
    },
    ProcessExited {
        plugin_id: String,
        code: Option<i32>,
    },
    Protocol(String),
    Io(String),
}

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Discovery(msg) => write!(f, "{msg}"),
            Self::Spawn(msg) => write!(f, "{msg}"),
            Self::UnknownPlugin(id) => write!(f, "unknown plugin id: {id}"),
            Self::AlreadyRunning(id) => write!(f, "plugin is already running: {id}"),
            Self::NotRunning(id) => write!(f, "plugin is not running: {id}"),
            Self::Timeout {
                plugin_id,
                timeout_ms,
            } => write!(
                f,
                "timed out waiting for plugin {plugin_id} request after {timeout_ms}ms"
            ),
            Self::ProcessExited { plugin_id, code } => {
                write!(f, "plugin process exited: {plugin_id} (code={code:?})")
            }
            Self::Protocol(msg) => write!(f, "{msg}"),
            Self::Io(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for RuntimeError {}

/// Discover plugin manifests from one or more search roots.
///
/// A root can be either a directory (recursively searched for `plugin.toml`)
/// or a direct path to a `plugin.toml` file.
pub fn discover_plugins(search_roots: &[PathBuf]) -> Result<Vec<DiscoveredPlugin>> {
    let mut manifest_paths = Vec::new();
    for root in search_roots {
        collect_manifest_paths(root, &mut manifest_paths)
            .with_context(|| format!("failed to scan plugin root {}", root.display()))?;
    }
    manifest_paths.sort();

    let mut discovered = Vec::new();
    let mut seen_ids: BTreeMap<String, PathBuf> = BTreeMap::new();

    for path in manifest_paths {
        let manifest = PluginManifest::from_path(&path)
            .with_context(|| format!("failed to load plugin manifest {}", path.display()))?;

        if let Some(previous) = seen_ids.insert(manifest.id.clone(), path.clone()) {
            bail!(
                "duplicate plugin id {:?} in manifests {} and {}",
                manifest.id,
                previous.display(),
                path.display()
            );
        }

        discovered.push(DiscoveredPlugin {
            manifest_path: path,
            manifest,
        });
    }

    discovered.sort_by(|left, right| left.manifest.id.cmp(&right.manifest.id));
    Ok(discovered)
}

/// Registry of discovered plugins and live runtime sessions.
pub struct PluginRuntime {
    plugins: BTreeMap<String, RegisteredPlugin>,
}

impl PluginRuntime {
    /// Build a runtime registry from plugin search roots.
    pub fn from_search_roots(search_roots: &[PathBuf]) -> std::result::Result<Self, RuntimeError> {
        let discovered = discover_plugins(search_roots)
            .map_err(|err| RuntimeError::Discovery(format!("plugin discovery failed: {err}")))?;
        Self::register_discovered(discovered)
    }

    /// Build a runtime registry from discovered manifests.
    pub fn register_discovered(
        discovered: Vec<DiscoveredPlugin>,
    ) -> std::result::Result<Self, RuntimeError> {
        let mut plugins = BTreeMap::new();

        for item in discovered {
            if plugins.contains_key(&item.manifest.id) {
                return Err(RuntimeError::Discovery(format!(
                    "duplicate plugin id in discovered set: {}",
                    item.manifest.id
                )));
            }

            let policy = policy_from_manifest(&item.manifest).map_err(|err| {
                RuntimeError::Discovery(format!(
                    "plugin {} failed compatibility/permission validation: {err}",
                    item.manifest.id
                ))
            })?;

            plugins.insert(
                item.manifest.id.clone(),
                RegisteredPlugin {
                    manifest_path: item.manifest_path,
                    manifest: item.manifest,
                    policy,
                    session: None,
                },
            );
        }

        Ok(Self { plugins })
    }

    /// Return all registered plugin IDs in sorted order.
    pub fn plugin_ids(&self) -> Vec<&str> {
        self.plugins.keys().map(String::as_str).collect()
    }

    /// Start a plugin process and complete handshake.
    pub fn start(
        &mut self,
        plugin_id: &str,
        timeout: Duration,
    ) -> std::result::Result<HandshakeResult, RuntimeError> {
        let plugin = self
            .plugins
            .get_mut(plugin_id)
            .ok_or_else(|| RuntimeError::UnknownPlugin(plugin_id.to_string()))?;

        if plugin.session.is_some() {
            return Err(RuntimeError::AlreadyRunning(plugin_id.to_string()));
        }

        let mut session = PluginSession::spawn(
            &plugin.manifest_path,
            plugin.manifest.clone(),
            plugin.policy.clone(),
        )?;

        let handshake = match session.complete_handshake(timeout) {
            Ok(result) => result,
            Err(err) => {
                session.shutdown();
                return Err(err);
            }
        };

        plugin.session = Some(session);
        Ok(handshake)
    }

    /// Pump a single inbound request from a running plugin session.
    pub fn pump_next<H: HostBridge>(
        &mut self,
        plugin_id: &str,
        host: &mut H,
        timeout: Duration,
    ) -> std::result::Result<HandledRequest, RuntimeError> {
        let mut clear_session = false;
        let result = {
            let plugin = self
                .plugins
                .get_mut(plugin_id)
                .ok_or_else(|| RuntimeError::UnknownPlugin(plugin_id.to_string()))?;

            let session = plugin
                .session
                .as_mut()
                .ok_or_else(|| RuntimeError::NotRunning(plugin_id.to_string()))?;

            let result = session.pump_next(host, timeout);
            if matches!(result, Err(RuntimeError::ProcessExited { .. })) {
                clear_session = true;
            }
            result
        };

        if clear_session {
            if let Some(plugin) = self.plugins.get_mut(plugin_id) {
                plugin.session = None;
            }
        }

        result
    }

    /// Broadcast a host event notification to subscribed running plugins.
    ///
    /// Returns the number of plugin sessions that received the event.
    pub fn broadcast_event(
        &mut self,
        category: EventCategory,
        tag: Option<&str>,
        payload: Value,
    ) -> std::result::Result<usize, RuntimeError> {
        let mut delivered = 0usize;
        let mut crashed = Vec::new();

        for (plugin_id, plugin) in &mut self.plugins {
            let Some(session) = plugin.session.as_mut() else {
                continue;
            };

            match session.dispatch_event(category, tag, payload.clone()) {
                Ok(true) => delivered += 1,
                Ok(false) => {}
                Err(RuntimeError::ProcessExited { .. }) => crashed.push(plugin_id.clone()),
                Err(err) => return Err(err),
            }
        }

        for plugin_id in crashed {
            if let Some(plugin) = self.plugins.get_mut(&plugin_id) {
                plugin.session = None;
            }
        }

        Ok(delivered)
    }

    /// Stop one running plugin process.
    pub fn shutdown_plugin(&mut self, plugin_id: &str) -> std::result::Result<(), RuntimeError> {
        let plugin = self
            .plugins
            .get_mut(plugin_id)
            .ok_or_else(|| RuntimeError::UnknownPlugin(plugin_id.to_string()))?;

        plugin.session = None;
        Ok(())
    }

    /// Stop all running plugin processes.
    pub fn shutdown_all(&mut self) {
        for plugin in self.plugins.values_mut() {
            plugin.session = None;
        }
    }
}

struct RegisteredPlugin {
    manifest_path: PathBuf,
    manifest: PluginManifest,
    policy: PermissionPolicy,
    session: Option<PluginSession>,
}

struct PluginSession {
    plugin_id: String,
    manifest: PluginManifest,
    policy: PermissionPolicy,
    child: Child,
    stdin: ChildStdin,
    reader_rx: Receiver<ReaderEvent>,
    handshake_complete: bool,
    subscriptions: BTreeSet<String>,
}

impl PluginSession {
    fn spawn(
        manifest_path: &Path,
        manifest: PluginManifest,
        policy: PermissionPolicy,
    ) -> std::result::Result<Self, RuntimeError> {
        let manifest_dir = manifest_path.parent().ok_or_else(|| {
            RuntimeError::Spawn(format!(
                "manifest has no parent directory: {}",
                manifest_path.display()
            ))
        })?;

        let entrypoint = manifest_dir.join(&manifest.runtime.entrypoint);
        if !entrypoint.exists() {
            return Err(RuntimeError::Spawn(format!(
                "plugin entrypoint does not exist: {}",
                entrypoint.display()
            )));
        }

        let mut command = if let Some(runtime_command) = &manifest.runtime.command {
            let mut command = Command::new(runtime_command);
            command.args(&manifest.runtime.args);
            command.arg(&entrypoint);
            command
        } else {
            let mut command = Command::new(&entrypoint);
            command.args(&manifest.runtime.args);
            command
        };

        command
            .current_dir(manifest_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());

        let mut child = command.spawn().map_err(|err| {
            RuntimeError::Spawn(format!(
                "failed to spawn plugin {} process: {err}",
                manifest.id
            ))
        })?;

        let stdin = child.stdin.take().ok_or_else(|| {
            RuntimeError::Spawn(format!(
                "failed to capture plugin {} stdin pipe",
                manifest.id
            ))
        })?;

        let stdout = child.stdout.take().ok_or_else(|| {
            RuntimeError::Spawn(format!(
                "failed to capture plugin {} stdout pipe",
                manifest.id
            ))
        })?;

        Ok(Self {
            plugin_id: manifest.id.clone(),
            manifest,
            policy,
            child,
            stdin,
            reader_rx: spawn_reader(stdout),
            handshake_complete: false,
            subscriptions: BTreeSet::new(),
        })
    }

    fn complete_handshake(
        &mut self,
        timeout: Duration,
    ) -> std::result::Result<HandshakeResult, RuntimeError> {
        let request = self.next_request(timeout)?;

        if request.method != HANDSHAKE_METHOD {
            let error = JsonRpcError {
                code: error_code::INVALID_PARAMS,
                message: format!(
                    "first plugin request must be {HANDSHAKE_METHOD}, got {}",
                    request.method
                ),
                data: None,
            };
            self.send_error_response(request.id.clone(), error)?;
            return Err(RuntimeError::Protocol(format!(
                "plugin {} did not start with {HANDSHAKE_METHOD}",
                self.plugin_id
            )));
        }

        let params: HandshakeParams = match parse_params(&request) {
            Ok(params) => params,
            Err(error) => {
                self.send_error_response(request.id.clone(), error)?;
                return Err(RuntimeError::Protocol(format!(
                    "invalid handshake params from plugin {}",
                    self.plugin_id
                )));
            }
        };

        if params.plugin_id != self.manifest.id {
            let error = JsonRpcError {
                code: error_code::INVALID_PARAMS,
                message: format!(
                    "handshake plugin_id mismatch: manifest={} request={}",
                    self.manifest.id, params.plugin_id
                ),
                data: None,
            };
            self.send_error_response(request.id.clone(), error)?;
            return Err(RuntimeError::Protocol(format!(
                "handshake plugin_id mismatch for {}",
                self.plugin_id
            )));
        }

        if params.plugin_version != self.manifest.version {
            let error = JsonRpcError {
                code: error_code::INVALID_PARAMS,
                message: format!(
                    "handshake plugin_version mismatch: manifest={} request={}",
                    self.manifest.version, params.plugin_version
                ),
                data: None,
            };
            self.send_error_response(request.id.clone(), error)?;
            return Err(RuntimeError::Protocol(format!(
                "handshake plugin_version mismatch for {}",
                self.plugin_id
            )));
        }

        match build_handshake_result(&params) {
            Ok(result) => {
                self.send_result_response(request.id, &result)?;
                self.handshake_complete = true;
                Ok(result)
            }
            Err(error) => {
                self.send_error_response(request.id.clone(), error.to_jsonrpc_error())?;
                Err(RuntimeError::Protocol(format!(
                    "handshake negotiation failed for plugin {}: {error}",
                    self.plugin_id
                )))
            }
        }
    }

    fn pump_next<H: HostBridge>(
        &mut self,
        host: &mut H,
        timeout: Duration,
    ) -> std::result::Result<HandledRequest, RuntimeError> {
        let request = self.next_request(timeout)?;
        self.handle_request(request, host)
    }

    fn handle_request<H: HostBridge>(
        &mut self,
        request: JsonRpcRequestEnvelope,
        host: &mut H,
    ) -> std::result::Result<HandledRequest, RuntimeError> {
        if request.jsonrpc != JSONRPC_VERSION {
            let error = JsonRpcError {
                code: error_code::INVALID_PARAMS,
                message: format!("unsupported jsonrpc version: {}", request.jsonrpc),
                data: None,
            };
            self.send_error_response(request.id.clone(), error)?;
            return Ok(HandledRequest {
                plugin_id: self.plugin_id.clone(),
                method: request.method,
                responded_with_error: true,
            });
        }

        if !self.handshake_complete && request.method != HANDSHAKE_METHOD {
            let error = JsonRpcError {
                code: error_code::PLUGIN_UNAVAILABLE,
                message: format!(
                    "plugin {} must complete {HANDSHAKE_METHOD} first",
                    self.plugin_id
                ),
                data: None,
            };
            self.send_error_response(request.id.clone(), error)?;
            return Ok(HandledRequest {
                plugin_id: self.plugin_id.clone(),
                method: request.method,
                responded_with_error: true,
            });
        }

        if self.handshake_complete && request.method == HANDSHAKE_METHOD {
            let error = JsonRpcError {
                code: error_code::INVALID_PARAMS,
                message: format!("{HANDSHAKE_METHOD} already completed"),
                data: None,
            };
            self.send_error_response(request.id.clone(), error)?;
            return Ok(HandledRequest {
                plugin_id: self.plugin_id.clone(),
                method: request.method,
                responded_with_error: true,
            });
        }

        let method = request.method.clone();

        let responded_with_error = match method.as_str() {
            GET_SNAPSHOT_METHOD => {
                if let Err(error) = parse_params::<GetSnapshotParams>(&request) {
                    self.send_error_response(request.id.clone(), error)?;
                    true
                } else {
                    match host.state_snapshot() {
                        Ok(snapshot) => {
                            self.send_result_response(request.id.clone(), &snapshot)?;
                            false
                        }
                        Err(err) => {
                            self.send_error_response(
                                request.id.clone(),
                                host_unavailable_error(err),
                            )?;
                            true
                        }
                    }
                }
            }
            SUBSCRIBE_METHOD => match parse_params::<SubscribeParams>(&request) {
                Ok(params) => match self.policy.authorize_subscriptions(&params.categories) {
                    Ok(authorized) => {
                        for category in authorized {
                            self.subscriptions.insert(category.as_str().to_string());
                        }
                        let result = SubscriptionResult {
                            subscribed: self.current_subscriptions(),
                        };
                        self.send_result_response(request.id.clone(), &result)?;
                        false
                    }
                    Err(err) => {
                        self.send_error_response(request.id.clone(), err.to_jsonrpc_error())?;
                        true
                    }
                },
                Err(error) => {
                    self.send_error_response(request.id.clone(), error)?;
                    true
                }
            },
            UNSUBSCRIBE_METHOD => match parse_params::<SubscribeParams>(&request) {
                Ok(params) => match self.policy.authorize_subscriptions(&params.categories) {
                    Ok(authorized) => {
                        for category in authorized {
                            self.subscriptions.remove(category.as_str());
                        }
                        let result = SubscriptionResult {
                            subscribed: self.current_subscriptions(),
                        };
                        self.send_result_response(request.id.clone(), &result)?;
                        false
                    }
                    Err(err) => {
                        self.send_error_response(request.id.clone(), err.to_jsonrpc_error())?;
                        true
                    }
                },
                Err(error) => {
                    self.send_error_response(request.id.clone(), error)?;
                    true
                }
            },
            INVOKE_COMMAND_METHOD => match parse_params::<InvokeCommandParams>(&request) {
                Ok(params) => match self.policy.authorize_invoke_command(&params) {
                    Ok(()) => match host.invoke_command(params) {
                        Ok(result) => {
                            self.send_result_response(request.id.clone(), &result)?;
                            false
                        }
                        Err(err) => {
                            self.send_error_response(
                                request.id.clone(),
                                host_unavailable_error(err),
                            )?;
                            true
                        }
                    },
                    Err(err) => {
                        self.send_error_response(request.id.clone(), err.to_jsonrpc_error())?;
                        true
                    }
                },
                Err(error) => {
                    self.send_error_response(request.id.clone(), error)?;
                    true
                }
            },
            PUBLISH_EVENT_METHOD => match parse_params::<PublishEventParams>(&request) {
                Ok(params) => match self.policy.authorize_publish_event(&params) {
                    Ok(()) => match host.publish_event(params) {
                        Ok(result) => {
                            self.send_result_response(request.id.clone(), &result)?;
                            false
                        }
                        Err(err) => {
                            self.send_error_response(
                                request.id.clone(),
                                host_unavailable_error(err),
                            )?;
                            true
                        }
                    },
                    Err(err) => {
                        self.send_error_response(request.id.clone(), err.to_jsonrpc_error())?;
                        true
                    }
                },
                Err(error) => {
                    self.send_error_response(request.id.clone(), error)?;
                    true
                }
            },
            _ => {
                let error = JsonRpcError {
                    code: error_code::INVALID_PARAMS,
                    message: format!("unsupported method: {}", request.method),
                    data: None,
                };
                self.send_error_response(request.id.clone(), error)?;
                true
            }
        };

        Ok(HandledRequest {
            plugin_id: self.plugin_id.clone(),
            method,
            responded_with_error,
        })
    }

    fn dispatch_event(
        &mut self,
        category: EventCategory,
        tag: Option<&str>,
        payload: Value,
    ) -> std::result::Result<bool, RuntimeError> {
        if !self.handshake_complete || !self.subscriptions.contains(category.as_str()) {
            return Ok(false);
        }

        let notification = JsonRpcNotificationEnvelope {
            jsonrpc: JSONRPC_VERSION.to_string(),
            method: EVENT_NOTIFICATION_METHOD.to_string(),
            params: serde_json::to_value(EventNotificationParams {
                category,
                tag: tag.map(str::to_string),
                payload,
            })
            .map_err(|err| {
                RuntimeError::Protocol(format!("failed to encode event payload: {err}"))
            })?,
        };

        self.send_json_line(&notification)?;
        Ok(true)
    }

    fn current_subscriptions(&self) -> Vec<EventCategory> {
        self.subscriptions
            .iter()
            .filter_map(|name| event_category_from_name(name))
            .collect()
    }

    fn next_request(
        &mut self,
        timeout: Duration,
    ) -> std::result::Result<JsonRpcRequestEnvelope, RuntimeError> {
        match self.reader_rx.recv_timeout(timeout) {
            Ok(ReaderEvent::Request(request)) => Ok(request),
            Ok(ReaderEvent::ProtocolError(message)) => Err(RuntimeError::Protocol(format!(
                "plugin {} protocol error: {message}",
                self.plugin_id
            ))),
            Ok(ReaderEvent::IoError(message)) => Err(RuntimeError::Io(format!(
                "plugin {} stdout read error: {message}",
                self.plugin_id
            ))),
            Ok(ReaderEvent::Eof) => Err(self.process_exited_error()),
            Err(RecvTimeoutError::Timeout) => {
                if let Some(status) = self.child.try_wait().map_err(|err| {
                    RuntimeError::Io(format!(
                        "failed to poll plugin {} process status: {err}",
                        self.plugin_id
                    ))
                })? {
                    return Err(RuntimeError::ProcessExited {
                        plugin_id: self.plugin_id.clone(),
                        code: status.code(),
                    });
                }

                Err(RuntimeError::Timeout {
                    plugin_id: self.plugin_id.clone(),
                    timeout_ms: timeout.as_millis() as u64,
                })
            }
            Err(RecvTimeoutError::Disconnected) => Err(self.process_exited_error()),
        }
    }

    fn send_result_response<T: Serialize>(
        &mut self,
        id: RequestId,
        result: &T,
    ) -> std::result::Result<(), RuntimeError> {
        let result = serde_json::to_value(result).map_err(|err| {
            RuntimeError::Protocol(format!("failed to encode JSON-RPC result: {err}"))
        })?;
        let response = JsonRpcResponseEnvelope {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id,
            result: Some(result),
            error: None,
        };
        self.send_json_line(&response)
    }

    fn send_error_response(
        &mut self,
        id: RequestId,
        error: JsonRpcError,
    ) -> std::result::Result<(), RuntimeError> {
        let response = JsonRpcResponseEnvelope {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id,
            result: None,
            error: Some(error),
        };
        self.send_json_line(&response)
    }

    fn send_json_line<T: Serialize>(
        &mut self,
        payload: &T,
    ) -> std::result::Result<(), RuntimeError> {
        let encoded = serde_json::to_string(payload).map_err(|err| {
            RuntimeError::Protocol(format!("failed to encode JSON-RPC payload: {err}"))
        })?;

        self.stdin
            .write_all(encoded.as_bytes())
            .map_err(|err| self.io_error(err))?;
        self.stdin
            .write_all(b"\n")
            .map_err(|err| self.io_error(err))?;
        self.stdin.flush().map_err(|err| self.io_error(err))?;
        Ok(())
    }

    fn io_error(&mut self, error: std::io::Error) -> RuntimeError {
        match self.child.try_wait() {
            Ok(Some(status)) => RuntimeError::ProcessExited {
                plugin_id: self.plugin_id.clone(),
                code: status.code(),
            },
            Ok(None) | Err(_) => {
                RuntimeError::Io(format!("plugin {} stdio error: {error}", self.plugin_id))
            }
        }
    }

    fn process_exited_error(&mut self) -> RuntimeError {
        match self.child.try_wait() {
            Ok(Some(status)) => RuntimeError::ProcessExited {
                plugin_id: self.plugin_id.clone(),
                code: status.code(),
            },
            Ok(None) => RuntimeError::Io(format!(
                "plugin {} request stream ended unexpectedly",
                self.plugin_id
            )),
            Err(err) => RuntimeError::Io(format!(
                "failed to poll plugin {} process status: {err}",
                self.plugin_id
            )),
        }
    }

    fn shutdown(&mut self) {
        match self.child.try_wait() {
            Ok(Some(_)) => {}
            Ok(None) => {
                let _ = self.child.kill();
                let _ = self.child.wait();
            }
            Err(_) => {}
        }
    }
}

impl Drop for PluginSession {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct JsonRpcRequestEnvelope {
    jsonrpc: String,
    id: RequestId,
    method: String,
    #[serde(default)]
    params: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct JsonRpcResponseEnvelope {
    jsonrpc: String,
    id: RequestId,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct JsonRpcNotificationEnvelope {
    jsonrpc: String,
    method: String,
    params: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct EventNotificationParams {
    category: EventCategory,
    #[serde(skip_serializing_if = "Option::is_none")]
    tag: Option<String>,
    payload: Value,
}

enum ReaderEvent {
    Request(JsonRpcRequestEnvelope),
    ProtocolError(String),
    IoError(String),
    Eof,
}

fn spawn_reader(stdout: ChildStdout) -> Receiver<ReaderEvent> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            match line {
                Ok(line) => {
                    if line.trim().is_empty() {
                        continue;
                    }

                    let parsed = serde_json::from_str::<JsonRpcRequestEnvelope>(&line)
                        .map_err(|err| format!("invalid JSON-RPC request ({err}): {line}"));

                    match parsed {
                        Ok(request) => {
                            if tx.send(ReaderEvent::Request(request)).is_err() {
                                return;
                            }
                        }
                        Err(message) => {
                            let _ = tx.send(ReaderEvent::ProtocolError(message));
                            return;
                        }
                    }
                }
                Err(err) => {
                    let _ = tx.send(ReaderEvent::IoError(err.to_string()));
                    return;
                }
            }
        }

        let _ = tx.send(ReaderEvent::Eof);
    });
    rx
}

fn parse_params<T: DeserializeOwned>(
    request: &JsonRpcRequestEnvelope,
) -> std::result::Result<T, JsonRpcError> {
    serde_json::from_value(request.params.clone()).map_err(|err| JsonRpcError {
        code: error_code::INVALID_PARAMS,
        message: format!("invalid params for {}: {err}", request.method),
        data: None,
    })
}

fn host_unavailable_error(err: anyhow::Error) -> JsonRpcError {
    JsonRpcError {
        code: error_code::PLUGIN_UNAVAILABLE,
        message: format!("host operation failed: {err}"),
        data: None,
    }
}

fn event_category_from_name(name: &str) -> Option<EventCategory> {
    match name {
        "tick" => Some(EventCategory::Tick),
        "resize" => Some(EventCategory::Resize),
        "module_lifecycle" => Some(EventCategory::ModuleLifecycle),
        "telemetry" => Some(EventCategory::Telemetry),
        "custom" => Some(EventCategory::Custom),
        _ => None,
    }
}

fn collect_manifest_paths(root: &Path, paths: &mut Vec<PathBuf>) -> Result<()> {
    if !root.exists() {
        return Ok(());
    }

    if root.is_file() {
        if root.file_name().and_then(|name| name.to_str()) == Some("plugin.toml") {
            paths.push(root.to_path_buf());
        }
        return Ok(());
    }

    let mut stack = vec![root.to_path_buf()];
    while let Some(path) = stack.pop() {
        for entry in fs::read_dir(&path)
            .with_context(|| format!("failed to read directory {}", path.display()))?
        {
            let entry = entry
                .with_context(|| format!("failed to read directory entry in {}", path.display()))?;
            let child = entry.path();
            if child.is_dir() {
                stack.push(child);
                continue;
            }

            if child.file_name().and_then(|name| name.to_str()) == Some("plugin.toml") {
                paths.push(child);
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{Instant, SystemTime, UNIX_EPOCH};

    use serde_json::json;

    use crate::protocol::HOST_API_VERSION;

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    struct TestDir {
        path: PathBuf,
    }

    impl TestDir {
        fn new(name: &str) -> Self {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let counter = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "spud-remote-{name}-{}-{nanos}-{counter}",
                std::process::id()
            ));
            fs::create_dir_all(&path).unwrap();
            Self { path }
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    #[derive(Default)]
    struct MockHost {
        snapshot_calls: usize,
        invoked_commands: Vec<String>,
        published_tags: Vec<String>,
    }

    impl HostBridge for MockHost {
        fn state_snapshot(&mut self) -> Result<StateSnapshot> {
            self.snapshot_calls += 1;
            Ok(StateSnapshot {
                active_module: Some(crate::protocol::ActiveModule {
                    id: "hello".to_string(),
                    title: "Hello".to_string(),
                }),
                status_line: "OK".to_string(),
                uptime_seconds: 42,
                tps: 10.0,
                telemetry: Vec::new(),
            })
        }

        fn invoke_command(&mut self, params: InvokeCommandParams) -> Result<InvokeCommandResult> {
            self.invoked_commands.push(params.command.clone());
            Ok(InvokeCommandResult {
                lines: vec![format!("ok:{}", params.command)],
            })
        }

        fn publish_event(&mut self, params: PublishEventParams) -> Result<PublishEventResult> {
            self.published_tags.push(params.tag);
            Ok(PublishEventResult { accepted: true })
        }
    }

    fn write_plugin_manifest(
        dir: &Path,
        plugin_id: &str,
        entrypoint: &str,
        commands: &[&str],
        event_tags: &[&str],
        subscriptions: &[&str],
    ) {
        let commands = toml_array(commands);
        let event_tags = toml_array(event_tags);
        let subscriptions = toml_array(subscriptions);

        let manifest = format!(
            r#"
id = "{plugin_id}"
name = "Fixture Plugin"
version = "0.1.0"

[runtime]
entrypoint = "{entrypoint}"
command = "sh"
args = []

[compatibility]
host_api = "^1.0.0"

[permissions]
commands = {commands}
event_tags = {event_tags}
subscriptions = {subscriptions}
"#
        );

        fs::write(dir.join("plugin.toml"), manifest).unwrap();
    }

    fn toml_array(values: &[&str]) -> String {
        format!(
            "[{}]",
            values
                .iter()
                .map(|value| format!("\"{value}\""))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }

    fn wait_for_transcript(path: &Path, min_lines: usize) -> Vec<String> {
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            if let Ok(raw) = fs::read_to_string(path) {
                let lines = raw
                    .lines()
                    .map(|line| line.to_string())
                    .collect::<Vec<String>>();
                if lines.len() >= min_lines {
                    return lines;
                }
            }

            if Instant::now() > deadline {
                panic!(
                    "timed out waiting for transcript {} to reach {min_lines} lines",
                    path.display()
                );
            }

            thread::sleep(Duration::from_millis(10));
        }
    }

    #[test]
    fn discover_plugins_rejects_duplicate_plugin_ids() {
        let root = TestDir::new("duplicate-discovery");
        let plugin_a = root.path.join("a");
        let plugin_b = root.path.join("b");
        fs::create_dir_all(&plugin_a).unwrap();
        fs::create_dir_all(&plugin_b).unwrap();

        write_plugin_manifest(&plugin_a, "spud.dup", "a.sh", &[], &[], &[]);
        write_plugin_manifest(&plugin_b, "spud.dup", "b.sh", &[], &[], &[]);

        let err = discover_plugins(std::slice::from_ref(&root.path)).unwrap_err();
        assert!(err.to_string().contains("duplicate plugin id"));
    }

    #[cfg(unix)]
    #[test]
    fn runtime_starts_handshakes_and_bridges_requests() {
        let root = TestDir::new("bridge-flow");
        let plugin_dir = root.path.join("plugin");
        fs::create_dir_all(&plugin_dir).unwrap();

        let transcript = plugin_dir.join("transcript.log");
        let script = r#"#!/bin/sh
set -eu
TRANSCRIPT="__TRANSCRIPT__"

echo '{"jsonrpc":"2.0","id":1,"method":"spud.handshake","params":{"plugin_id":"spud.fixture","plugin_version":"0.1.0","supported_api_versions":"^1.0.0","requested_capabilities":[]}}'
IFS= read -r line
echo "$line" >> "$TRANSCRIPT"

echo '{"jsonrpc":"2.0","id":2,"method":"spud.state.get_snapshot","params":{}}'
IFS= read -r line
echo "$line" >> "$TRANSCRIPT"

echo '{"jsonrpc":"2.0","id":3,"method":"spud.host.invoke_command","params":{"command":"help","args":[]}}'
IFS= read -r line
echo "$line" >> "$TRANSCRIPT"

echo '{"jsonrpc":"2.0","id":4,"method":"spud.host.publish_event","params":{"tag":"plugin.metrics","payload":"{}"}}'
IFS= read -r line
echo "$line" >> "$TRANSCRIPT"

echo '{"jsonrpc":"2.0","id":5,"method":"spud.events.subscribe","params":{"categories":["tick"]}}'
IFS= read -r line
echo "$line" >> "$TRANSCRIPT"

IFS= read -r line
echo "$line" >> "$TRANSCRIPT"
"#
        .replace("__TRANSCRIPT__", &transcript.to_string_lossy());

        fs::write(plugin_dir.join("plugin.sh"), script).unwrap();
        write_plugin_manifest(
            &plugin_dir,
            "spud.fixture",
            "plugin.sh",
            &["help"],
            &["plugin.metrics"],
            &["tick"],
        );

        let mut runtime =
            PluginRuntime::from_search_roots(std::slice::from_ref(&root.path)).unwrap();
        assert_eq!(runtime.plugin_ids(), vec!["spud.fixture"]);

        let handshake = runtime
            .start("spud.fixture", Duration::from_secs(2))
            .unwrap();
        assert_eq!(handshake.selected_api_version, HOST_API_VERSION);

        let mut host = MockHost::default();
        let methods = (0..4)
            .map(|_| {
                runtime
                    .pump_next("spud.fixture", &mut host, Duration::from_secs(2))
                    .unwrap()
                    .method
            })
            .collect::<Vec<_>>();

        assert_eq!(
            methods,
            vec![
                GET_SNAPSHOT_METHOD.to_string(),
                INVOKE_COMMAND_METHOD.to_string(),
                PUBLISH_EVENT_METHOD.to_string(),
                SUBSCRIBE_METHOD.to_string(),
            ]
        );

        let delivered = runtime
            .broadcast_event(EventCategory::Tick, Some("tick"), json!({"now": 1}))
            .unwrap();
        assert_eq!(delivered, 1);

        let lines = wait_for_transcript(&transcript, 6);
        let handshake_response: Value = serde_json::from_str(&lines[0]).unwrap();
        let snapshot_response: Value = serde_json::from_str(&lines[1]).unwrap();
        let invoke_response: Value = serde_json::from_str(&lines[2]).unwrap();
        let publish_response: Value = serde_json::from_str(&lines[3]).unwrap();
        let subscribe_response: Value = serde_json::from_str(&lines[4]).unwrap();
        let notification: Value = serde_json::from_str(&lines[5]).unwrap();

        assert_eq!(
            handshake_response["result"]["selected_api_version"],
            HOST_API_VERSION
        );
        assert_eq!(snapshot_response["result"]["status_line"], "OK");
        assert_eq!(invoke_response["result"]["lines"][0], "ok:help");
        assert_eq!(publish_response["result"]["accepted"], true);
        assert_eq!(subscribe_response["result"]["subscribed"][0], "tick");
        assert_eq!(notification["method"], EVENT_NOTIFICATION_METHOD);
        assert_eq!(notification["params"]["category"], "tick");

        assert_eq!(host.snapshot_calls, 1);
        assert_eq!(host.invoked_commands, vec!["help".to_string()]);
        assert_eq!(host.published_tags, vec!["plugin.metrics".to_string()]);

        runtime.shutdown_all();
    }

    #[cfg(unix)]
    #[test]
    fn runtime_denies_unallowlisted_command_invocation() {
        let root = TestDir::new("deny-invoke");
        let plugin_dir = root.path.join("plugin");
        fs::create_dir_all(&plugin_dir).unwrap();

        let transcript = plugin_dir.join("transcript.log");
        let script = r#"#!/bin/sh
set -eu
TRANSCRIPT="__TRANSCRIPT__"

echo '{"jsonrpc":"2.0","id":1,"method":"spud.handshake","params":{"plugin_id":"spud.deny","plugin_version":"0.1.0","supported_api_versions":"^1.0.0","requested_capabilities":[]}}'
IFS= read -r line
echo "$line" >> "$TRANSCRIPT"

echo '{"jsonrpc":"2.0","id":2,"method":"spud.host.invoke_command","params":{"command":"quit","args":[]}}'
IFS= read -r line
echo "$line" >> "$TRANSCRIPT"
"#
        .replace("__TRANSCRIPT__", &transcript.to_string_lossy());

        fs::write(plugin_dir.join("plugin.sh"), script).unwrap();
        write_plugin_manifest(
            &plugin_dir,
            "spud.deny",
            "plugin.sh",
            &["help"],
            &["plugin.metrics"],
            &["tick"],
        );

        let mut runtime =
            PluginRuntime::from_search_roots(std::slice::from_ref(&root.path)).unwrap();
        runtime.start("spud.deny", Duration::from_secs(2)).unwrap();

        let mut host = MockHost::default();
        let handled = runtime
            .pump_next("spud.deny", &mut host, Duration::from_secs(2))
            .unwrap();
        assert_eq!(handled.method, INVOKE_COMMAND_METHOD);
        assert!(handled.responded_with_error);
        assert!(host.invoked_commands.is_empty());

        let lines = wait_for_transcript(&transcript, 2);
        let denied_response: Value = serde_json::from_str(&lines[1]).unwrap();
        assert_eq!(denied_response["error"]["code"], error_code::UNAUTHORIZED);

        runtime.shutdown_all();
    }

    #[cfg(unix)]
    #[test]
    fn crashed_plugin_is_reported_without_panicking_host() {
        let root = TestDir::new("crash-isolation");
        let plugin_dir = root.path.join("plugin");
        fs::create_dir_all(&plugin_dir).unwrap();

        fs::write(plugin_dir.join("plugin.sh"), "#!/bin/sh\nexit 17\n").unwrap();
        write_plugin_manifest(&plugin_dir, "spud.crash", "plugin.sh", &[], &[], &[]);

        let mut runtime =
            PluginRuntime::from_search_roots(std::slice::from_ref(&root.path)).unwrap();
        let err = runtime
            .start("spud.crash", Duration::from_millis(400))
            .unwrap_err();

        assert!(matches!(
            err,
            RuntimeError::ProcessExited {
                plugin_id,
                code: Some(17)
            } if plugin_id == "spud.crash"
        ));
    }
}
