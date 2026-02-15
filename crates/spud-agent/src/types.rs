use anyhow::{bail, Result};

/// A single ASCII animation frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AsciiFrame {
    /// Multi-line face content rendered in the HUD face panel.
    pub lines: Vec<String>,
}

impl AsciiFrame {
    /// Construct an [`AsciiFrame`] from static string lines.
    pub fn from_lines(lines: &[&str]) -> Self {
        Self {
            lines: lines.iter().map(|line| (*line).to_string()).collect(),
        }
    }
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

/// A complete face pack: per-mood ASCII animation frames.
#[derive(Debug, Clone)]
pub struct FacePack {
    /// Frames indexed by `[mood_ordinal][frame_index]`.
    pub frames: Vec<Vec<AsciiFrame>>,
    /// Number of animation frames per mood.
    pub frames_per_mood: usize,
}

impl FacePack {
    /// Build a validated face pack.
    ///
    /// Requires one mood entry per [`Mood`] and the same number of frames for
    /// each mood.
    pub fn new(frames: Vec<Vec<AsciiFrame>>) -> Result<Self> {
        if frames.len() != Mood::COUNT {
            bail!(
                "face pack must define {} moods, got {}",
                Mood::COUNT,
                frames.len()
            );
        }

        let frames_per_mood = frames[0].len();
        if frames_per_mood == 0 {
            bail!("face pack must contain at least one frame per mood");
        }

        for (idx, mood_frames) in frames.iter().enumerate() {
            if mood_frames.len() != frames_per_mood {
                bail!(
                    "mood index {idx} has {} frames, expected {frames_per_mood}",
                    mood_frames.len()
                );
            }
        }

        Ok(Self {
            frames,
            frames_per_mood,
        })
    }
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

    #[test]
    fn face_pack_rejects_missing_mood_rows() {
        let frames = vec![vec![AsciiFrame::from_lines(&["x"])]];
        let err = FacePack::new(frames).unwrap_err();
        assert!(err.to_string().contains("must define"));
    }

    #[test]
    fn face_pack_rejects_zero_frames() {
        let mut frames = Vec::new();
        for _ in 0..Mood::COUNT {
            frames.push(Vec::new());
        }
        let err = FacePack::new(frames).unwrap_err();
        assert!(err.to_string().contains("at least one frame"));
    }

    #[test]
    fn face_pack_rejects_inconsistent_frame_counts() {
        let mut frames = Vec::new();
        frames.push(vec![AsciiFrame::from_lines(&["a"])]);
        for _ in 1..Mood::COUNT {
            frames.push(vec![
                AsciiFrame::from_lines(&["a"]),
                AsciiFrame::from_lines(&["b"]),
            ]);
        }
        let err = FacePack::new(frames).unwrap_err();
        assert!(err.to_string().contains("expected"));
    }
}
