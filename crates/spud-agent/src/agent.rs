use std::path::Path;
use std::time::Instant;

use anyhow::Result;

use crate::loader;
use crate::mood::MoodEngine;
use crate::types::{FacePack, Frame, Mood};

const DEFAULT_SHEET_PNG: &[u8] = include_bytes!("../../../assets/faces/default/sheet.png");
const DEFAULT_SHEET_JSON: &[u8] = include_bytes!("../../../assets/faces/default/face.json");

/// The SPUD agent face — owns a face pack and drives mood animation.
///
/// Consumers call [`tick`](Self::tick) each frame and
/// [`current_frame`](Self::current_frame) to get the pixels to render.
pub struct Agent {
    engine: MoodEngine,
}

impl Agent {
    /// Load the default (embedded) face pack.
    pub fn load_default(now: Instant) -> Result<Self> {
        let pack = loader::load_from_bytes(DEFAULT_SHEET_PNG, DEFAULT_SHEET_JSON)?;
        Ok(Self::from_pack(pack, now))
    }

    /// Load a custom face pack from PNG and JSON files.
    pub fn load_from_files(png: &Path, json: &Path, now: Instant) -> Result<Self> {
        let pack = loader::load_from_files(png, json)?;
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
    pub fn current_frame(&self) -> &Frame {
        self.engine.current_frame()
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
        let frame = agent.current_frame();
        assert_eq!(frame.width, 128);
        assert_eq!(frame.height, 128);
    }
}
