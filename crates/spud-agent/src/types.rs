use std::collections::HashMap;

use serde::Deserialize;

/// A single animation frame: raw RGBA pixel data at a known resolution.
///
/// Stored as a flat `Vec<u8>` in row-major RGBA order (4 bytes per pixel)
/// so consumers don't need to depend on the `image` crate.
#[derive(Debug, Clone)]
pub struct Frame {
    /// Raw RGBA pixel data, length = `width * height * 4`.
    pub data: Vec<u8>,
    /// Frame width in pixels.
    pub width: u32,
    /// Frame height in pixels.
    pub height: u32,
}

/// The six moods a SPUD agent face can display.
///
/// Variant order matches the row indices in [`FacePack::frames`], so
/// `Mood as usize` is a valid index into the outer `Vec`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mood {
    /// Resting / idle expression.
    Neutral = 0,
    /// Cheerful expression.
    Happy = 1,
    /// Irritated expression.
    Angry = 2,
    /// Powered-up fire effect.
    GodMode = 3,
    /// Badly damaged expression.
    HurtRealBad = 4,
    /// Contemplative expression.
    Thinking = 5,
}

impl Mood {
    /// Total number of mood variants.
    pub const COUNT: usize = 6;

    /// All moods in ordinal order.
    pub const ALL: [Mood; Self::COUNT] = [
        Mood::Neutral,
        Mood::Happy,
        Mood::Angry,
        Mood::GodMode,
        Mood::HurtRealBad,
        Mood::Thinking,
    ];
}

/// A complete face pack: per-mood animation frames decoded from a sprite sheet.
#[derive(Debug, Clone)]
pub struct FacePack {
    /// Frames indexed by `[mood_ordinal][frame_index]`.
    pub frames: Vec<Vec<Frame>>,
    /// Number of animation frames per mood.
    pub frames_per_mood: usize,
    /// Width of each frame in pixels.
    pub frame_width: u32,
    /// Height of each frame in pixels.
    pub frame_height: u32,
}

/// JSON metadata for a face pack sprite sheet.
///
/// Deserialized from `face.json` alongside the PNG.
#[derive(Debug, Deserialize)]
pub(crate) struct FacePackMeta {
    /// `[width, height]` of each sprite frame.
    pub sprite_size: [u32; 2],
    /// Number of animation frames per mood row.
    pub frames_per_mood: usize,
    /// Maps mood name (e.g. `"neutral"`) to its row index in the sheet.
    pub moods: HashMap<String, usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mood_ordinal_values() {
        assert_eq!(Mood::Neutral as usize, 0);
        assert_eq!(Mood::Happy as usize, 1);
        assert_eq!(Mood::Angry as usize, 2);
        assert_eq!(Mood::GodMode as usize, 3);
        assert_eq!(Mood::HurtRealBad as usize, 4);
        assert_eq!(Mood::Thinking as usize, 5);
    }

    #[test]
    fn mood_all_matches_count() {
        assert_eq!(Mood::ALL.len(), Mood::COUNT);
    }
}
