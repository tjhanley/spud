use std::time::{Duration, Instant};

use crate::types::{FacePack, Frame, Mood};

/// Default interval between animation frames.
const DEFAULT_FRAME_INTERVAL: Duration = Duration::from_millis(300);

/// Drives mood state and animation frame cycling for an agent face.
///
/// Owns a [`FacePack`] and tracks the current mood, frame index, and
/// timing for automatic frame advancement on each [`tick`](Self::tick).
pub(crate) struct MoodEngine {
    pack: FacePack,
    mood: Mood,
    frame_index: usize,
    frame_interval: Duration,
    last_advance: Instant,
}

impl MoodEngine {
    /// Create a new engine starting at [`Mood::Neutral`], frame 0.
    pub fn new(pack: FacePack, now: Instant) -> Self {
        Self {
            pack,
            mood: Mood::Neutral,
            frame_index: 0,
            frame_interval: DEFAULT_FRAME_INTERVAL,
            last_advance: now,
        }
    }

    /// Switch to a different mood, resetting to frame 0.
    ///
    /// No-op if the mood is already current.
    pub fn set_mood(&mut self, mood: Mood, now: Instant) {
        if self.mood == mood {
            return;
        }
        self.mood = mood;
        self.frame_index = 0;
        self.last_advance = now;
    }

    /// Advance the animation clock. Moves to the next frame when
    /// [`frame_interval`](Self::frame_interval) has elapsed. Catches up
    /// if multiple intervals have passed since the last tick.
    pub fn tick(&mut self, now: Instant) {
        if let Some(mut dt) = now.checked_duration_since(self.last_advance) {
            while dt >= self.frame_interval {
                self.frame_index = (self.frame_index + 1) % self.pack.frames_per_mood;
                self.last_advance += self.frame_interval;
                dt -= self.frame_interval;
            }
        }
    }

    /// Returns the current animation frame.
    pub fn current_frame(&self) -> &Frame {
        &self.pack.frames[self.mood as usize][self.frame_index]
    }

    /// Returns the current mood.
    pub fn mood(&self) -> Mood {
        self.mood
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Frame;

    /// Build a minimal [`FacePack`] with 1×1 frames whose RGBA red channel
    /// encodes `mood * 16 + frame` for easy assertion.
    fn test_pack(frames_per_mood: usize) -> FacePack {
        let mut frames = Vec::new();
        for mood in 0..Mood::COUNT {
            let mut mood_frames = Vec::new();
            for f in 0..frames_per_mood {
                let tag = (mood * 16 + f) as u8;
                mood_frames.push(Frame {
                    data: vec![tag, 0, 0, 255],
                    width: 1,
                    height: 1,
                });
            }
            frames.push(mood_frames);
        }
        FacePack {
            frames,
            frames_per_mood,
            frame_width: 1,
            frame_height: 1,
        }
    }

    #[test]
    fn initial_state() {
        let now = Instant::now();
        let engine = MoodEngine::new(test_pack(3), now);
        assert_eq!(engine.mood(), Mood::Neutral);
        // Frame 0 of Neutral: tag = 0*16+0 = 0
        assert_eq!(engine.current_frame().data[0], 0);
    }

    #[test]
    fn mood_transition_resets_frame() {
        let now = Instant::now();
        let mut engine = MoodEngine::new(test_pack(3), now);

        // Advance past frame 0
        let t1 = now + Duration::from_millis(300);
        engine.tick(t1);
        assert_eq!(engine.current_frame().data[0], 0 * 16 + 1); // Neutral frame 1

        // Switch mood — should reset to frame 0
        let t2 = t1 + Duration::from_millis(10);
        engine.set_mood(Mood::Angry, t2);
        assert_eq!(engine.mood(), Mood::Angry);
        // Frame 0 of Angry: tag = 2*16+0 = 32
        assert_eq!(engine.current_frame().data[0], 2 * 16);
    }

    #[test]
    fn same_mood_is_noop() {
        let now = Instant::now();
        let mut engine = MoodEngine::new(test_pack(3), now);

        // Advance to frame 1
        let t1 = now + Duration::from_millis(300);
        engine.tick(t1);
        assert_eq!(engine.current_frame().data[0], 1); // Neutral frame 1

        // Setting same mood should NOT reset
        engine.set_mood(Mood::Neutral, t1);
        assert_eq!(engine.current_frame().data[0], 1); // still frame 1
    }

    #[test]
    fn frame_cycling() {
        let now = Instant::now();
        let mut engine = MoodEngine::new(test_pack(3), now);

        for (step, expected_frame) in [0u8, 1, 2, 0, 1].iter().enumerate() {
            let tag = engine.current_frame().data[0];
            assert_eq!(
                tag, *expected_frame,
                "step {step}: expected frame {expected_frame}"
            );
            engine.tick(now + Duration::from_millis(300 * (step as u64 + 1)));
        }
    }

    #[test]
    fn frame_wraps_around() {
        let now = Instant::now();
        let mut engine = MoodEngine::new(test_pack(3), now);

        // Advance through all 3 frames and wrap
        for i in 1..=3 {
            engine.tick(now + Duration::from_millis(300 * i));
        }
        // Should be back to frame 0
        assert_eq!(engine.current_frame().data[0], 0);
    }

    #[test]
    fn no_advance_before_interval() {
        let now = Instant::now();
        let mut engine = MoodEngine::new(test_pack(3), now);

        // Tick just before the interval
        engine.tick(now + Duration::from_millis(299));
        assert_eq!(engine.current_frame().data[0], 0); // still frame 0

        // Tick at exactly the interval
        engine.tick(now + Duration::from_millis(300));
        assert_eq!(engine.current_frame().data[0], 1); // frame 1
    }

    #[test]
    fn tick_catches_up_on_long_gap() {
        let now = Instant::now();
        let mut engine = MoodEngine::new(test_pack(3), now);

        // Skip 750ms in one tick — should advance 2 frames (600ms worth), not just 1
        engine.tick(now + Duration::from_millis(750));
        assert_eq!(engine.current_frame().data[0], 2); // frame 2, not 1
    }
}
