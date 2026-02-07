//! Face pack loader and mood state machine for the SPUD agent.
//!
//! This crate decodes a PNG sprite sheet + JSON metadata into per-mood
//! animation frames, and drives a mood state machine that advances
//! frames on a timer.
//!
//! # Quick start
//!
//! ```no_run
//! use std::time::Instant;
//! use spud_agent::{Agent, Mood};
//!
//! let mut agent = Agent::load_default(Instant::now()).unwrap();
//! agent.set_mood(Mood::GodMode, Instant::now());
//! agent.tick(Instant::now());
//! let _frame = agent.current_frame();
//! ```

mod agent;
mod loader;
mod mood;
mod types;

pub use agent::Agent;
pub use types::{FacePack, Frame, Mood};
