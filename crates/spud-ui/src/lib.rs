//! TUI rendering layer for SPUD.
//!
//! Provides the Doom-style layout, shell chrome, and console overlay widgets.
//! All rendering uses [`ratatui`] — this crate owns the visual presentation
//! while [`spud_core`] owns the state.

pub mod console;
pub mod layout;
pub mod renderer;
pub mod shell;
