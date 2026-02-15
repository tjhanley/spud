use std::time::Instant;

use anyhow::Result;

use crate::default_pack;
use crate::mood::MoodEngine;
use crate::types::{AsciiFrame, FacePack, Mood};

/// The SPUD agent face — owns a face pack and drives mood animation.
///
/// Consumers call [`tick`](Self::tick) each frame and
/// [`current_frame`](Self::current_frame) to get the pixels to render.
pub struct Agent {
    engine: MoodEngine,
}

impl Agent {
    /// Load the default (embedded) ASCII face pack.
    pub fn load_default(now: Instant) -> Result<Self> {
        let pack = default_pack::load_default_pack()?;
        Ok(Self::from_pack(pack, now))
    }

    /// Create an agent from a pre-built [`FacePack`] (useful for testing).
    pub fn from_pack(pack: FacePack, now: Instant) -> Self {
        Self {
            engine: MoodEngine::new(pack, now),
        }
    }

    /// Advance the animation clock.
    pub fn tick(&mut self, now: Instant) {
        self.engine.tick(now);
    }

    /// Returns the current animation frame.
    pub fn current_frame(&self) -> &AsciiFrame {
        self.engine.current_frame()
    }

    /// Returns the current frame as text lines.
    pub fn current_frame_lines(&self) -> &[String] {
        &self.current_frame().lines
    }

    /// Switch to a different mood.
    pub fn set_mood(&mut self, mood: Mood, now: Instant) {
        self.engine.set_mood(mood, now);
    }

    /// Returns the current mood.
    pub fn mood(&self) -> Mood {
        self.engine.mood()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_default_succeeds() {
        let agent = Agent::load_default(Instant::now()).unwrap();
        assert_eq!(agent.mood(), Mood::Neutral);
        let frame = agent.current_frame_lines();
        assert!(!frame.is_empty());
        assert!(frame.iter().all(|line| !line.is_empty()));
    }
}
