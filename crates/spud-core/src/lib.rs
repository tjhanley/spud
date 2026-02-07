//! Core infrastructure for the SPUD runtime.
//!
//! This crate provides the foundational building blocks shared by the
//! application shell and all SPUD modules: an event bus, module registry,
//! drop-down console, command system, logging subsystem, and common types.

pub mod bus;
pub mod command;
pub mod console;
pub mod event;
pub mod fps;
pub mod logging;
pub mod module;
pub mod registry;
pub mod state;
