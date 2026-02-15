//! ASCII face pack and mood state machine for the SPUD agent.
//!
//! This crate provides embedded per-mood ASCII animation frames and drives
//! a mood state machine that advances frames on a timer.
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
//! let _frame = agent.current_frame_lines();
//! ```

mod agent;
mod default_pack;
mod mood;
mod types;

pub use agent::Agent;
pub use types::{AsciiFrame, FacePack, Mood};
