//! Configuration types and loaders for SPUD.
//!
//! This crate owns on-disk configuration schemas so runtime crates can share a
//! single source of truth.

pub mod plugin;

pub use plugin::{PluginCompatibility, PluginManifest, PluginPermissions, PluginRuntime};
