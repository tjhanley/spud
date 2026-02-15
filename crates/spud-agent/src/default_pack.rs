use anyhow::Result;

use crate::types::{AsciiFrame, FacePack};

fn frame(lines: &[&str]) -> AsciiFrame {
    AsciiFrame::from_lines(lines)
}

/// Build the embedded default ASCII face pack.
pub(crate) fn load_default_pack() -> Result<FacePack> {
    FacePack::new(vec![
        vec![
            frame(&["  .-----.  ", " | o   o | ", " |   -   | ", "  '-----'  "]),
            frame(&["  .-----.  ", " | -   - | ", " |   -   | ", "  '-----'  "]),
            frame(&["  .-----.  ", " | o   o | ", " |   o   | ", "  '-----'  "]),
        ],
        vec![
            frame(&["  .-----.  ", " | ^   ^ | ", " |  \\_/  | ", "  '-----'  "]),
            frame(&["  .-----.  ", " | ^   ^ | ", " |  \\__/ | ", "  '-----'  "]),
            frame(&["  .-----.  ", " | ^   ^ | ", " |  \\___/| ", "  '-----'  "]),
        ],
        vec![
            frame(&["  .-----.  ", " | >   < | ", " |   _   | ", "  '-----'  "]),
            frame(&["  .-----.  ", " | >   < | ", " |  ___  | ", "  '-----'  "]),
            frame(&["  .-----.  ", " | >   < | ", " |   _   | ", "  '-----'  "]),
        ],
        vec![
            frame(&["  .-***-.  ", " | *   * | ", " |  \\_/  | ", "  '-***-'  "]),
            frame(&["  .-***-.  ", " | *   * | ", " |  \\__/ | ", "  '-***-'  "]),
            frame(&["  .-***-.  ", " | *   * | ", " |  /_\\  | ", "  '-***-'  "]),
        ],
        vec![
            frame(&["  .-----.  ", " | x   x | ", " |   ~   | ", "  '-----'  "]),
            frame(&["  .-----.  ", " | x   x | ", " |  ___  | ", "  '-----'  "]),
            frame(&["  .-----.  ", " | x   x | ", " |   _   | ", "  '-----'  "]),
        ],
        vec![
            frame(&["  .-----.  ", " | o   - | ", " |   _   |?", "  '-----'  "]),
            frame(&["  .-----.  ", " | -   o | ", " |   _   |?", "  '-----'  "]),
            frame(&["  .-----.  ", " | o   o | ", " |   _   |?", "  '-----'  "]),
        ],
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Mood;

    #[test]
    fn default_pack_is_valid() {
        let pack = load_default_pack().unwrap();
        assert_eq!(pack.frames.len(), Mood::COUNT);
        assert_eq!(pack.frames_per_mood, 3);
    }
}
