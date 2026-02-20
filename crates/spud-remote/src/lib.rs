//! Remote/plugin runtime contract types for SPUD.
//!
//! This crate defines the versioned JSON-RPC host API contract and strongly
//! typed payloads shared by plugin-runtime implementation work.
//!
//! Runtime manager operations intentionally return a typed `RuntimeError` to
//! preserve deterministic caller control-flow in host pump loops.

pub mod permissions;
pub mod protocol;
pub mod runtime;
