use anyhow::Result;

use crate::types::{AsciiFrame, FacePack};

fn frame(lines: &[&str]) -> AsciiFrame {
    AsciiFrame::from_lines(lines)
}

// Pixel rows use palette keys consumed by `spud_ui::face`:
// '.' transparent, 'O' base orange, 'o' shadow orange, 'd' dark orange,
// 'k' near-black detail, 'h' highlight.
const NEUTRAL_1: &[&str] = &[
    "................",
    "....OOOOOOOO....",
    "...OOOOOOOOOO...",
    "..OOOkOOOOkOO...",
    "..OOOOOOOOOOOO..",
    ".OOOhOOOOOOhOOO.",
    ".OOOooooooooOOO.",
    ".OOOOOOOOOOOOOO.",
    "..OOOooooooOOO..",
    "..OOoOooooOoOO..",
    "...oo......oo...",
    "...oo......oo...",
];

const NEUTRAL_2: &[&str] = &[
    "................",
    "....OOOOOOOO....",
    "...OOOOOOOOOO...",
    "..OOOOkkkkOOOO..",
    "..OOOOOOOOOOOO..",
    ".OOOhOOOOOOhOOO.",
    ".OOOooooooooOOO.",
    ".OOOOOOOOOOOOOO.",
    "..OOOooooooOOO..",
    "..OOoOooooOoOO..",
    "...oo......oo...",
    "...oo......oo...",
];

const NEUTRAL_3: &[&str] = &[
    "................",
    "....OOOOOOOO....",
    "...OOOOOOOOOO...",
    "..OOOkOOkkOOOO..",
    "..OOOOOOOOOOOO..",
    ".OOOhOOOOOOhOOO.",
    ".OOOooooooooOOO.",
    ".OOOOOOOOOOOOOO.",
    "..OOOooooooOOO..",
    "..OOoOooooOoOO..",
    "...oo......oo...",
    "...oo......oo...",
];

const HAPPY_1: &[&str] = &[
    "................",
    "...hOOOOOOOOh...",
    "...OOOOOOOOOO...",
    "..OOOkOOOOkOO...",
    "..OOOOOOOOOOOO..",
    ".OOOhOOOOOOhOOO.",
    ".OOOOOOOOOOOOOO.",
    ".OOOooooooooOOO.",
    "..OOooooooooOO..",
    "...OOooooooOO...",
    "...oo......oo...",
    "...oo......oo...",
];

const HAPPY_2: &[&str] = &[
    "................",
    "...hOOOOOOOOh...",
    "...OOOOOOOOOO...",
    "..OOOOkkkkOOOO..",
    "..OOOOOOOOOOOO..",
    ".OOOhOOOOOOhOOO.",
    ".OOOOOOOOOOOOOO.",
    ".OOOooooooooOOO.",
    "..OOooooooooOO..",
    "...OOooooooOO...",
    "...oo......oo...",
    "...oo......oo...",
];

const HAPPY_3: &[&str] = &[
    "................",
    "...hOOOOOOOOh...",
    "...OOOOOOOOOO...",
    "..OOOkOOkkOOOO..",
    "..OOOOOOOOOOOO..",
    ".OOOhOOOOOOhOOO.",
    ".OOOOOOOOOOOOOO.",
    ".OOOooooooooOOO.",
    "..OOooooooooOO..",
    "...OOooooooOO...",
    "...oo......oo...",
    "...oo......oo...",
];

const ANGRY_1: &[&str] = &[
    "................",
    "...dddddddddd...",
    "...dOOOOOOOOd...",
    "..OOdkOOOOkdOO..",
    "..OOOOOOOOOOOO..",
    ".OOOdOOOOOOdOOO.",
    ".OOOddddddddOOO.",
    ".OOOOOOOOOOOOOO.",
    "..OOOddddddOOO..",
    "...OOdOOOOdOO...",
    "...oo......oo...",
    "...oo......oo...",
];

const ANGRY_2: &[&str] = &[
    "................",
    "...dddddddddd...",
    "...dOOOOOOOOd...",
    "..OOddkkkkddOO..",
    "..OOOOOOOOOOOO..",
    ".OOOdOOOOOOdOOO.",
    ".OOOddddddddOOO.",
    ".OOOOOOOOOOOOOO.",
    "..OOOddddddOOO..",
    "...OOdOOOOdOO...",
    "...oo......oo...",
    "...oo......oo...",
];

const ANGRY_3: &[&str] = &[
    "................",
    "...dddddddddd...",
    "...dOOOOOOOOd...",
    "..OOdkOOkkddOO..",
    "..OOOOOOOOOOOO..",
    ".OOOdOOOOOOdOOO.",
    ".OOOddddddddOOO.",
    ".OOOOOOOOOOOOOO.",
    "..OOOddddddOOO..",
    "...OOdOOOOdOO...",
    "...oo......oo...",
    "...oo......oo...",
];

const GODMODE_1: &[&str] = &[
    "....hhhhhhhh....",
    "...hOOOOOOOOh...",
    "..hOOOOOOOOOOh..",
    "..OOOkOOOOkOO...",
    "..OOOOOOOOOOOO..",
    ".OOOhOOOOOOhOOO.",
    ".OOOOOOOOOOOOOO.",
    ".OOOooooooooOOO.",
    "..OOOooooooOOO..",
    "..OOoOooooOoOO..",
    "..hho......ohh..",
    "..hho......ohh..",
];

const GODMODE_2: &[&str] = &[
    "...hhhhhhhhhh...",
    "...hOOOOOOOOh...",
    "..hOOOOOOOOOOh..",
    "..OOOOkkkkOOOO..",
    "..OOOOOOOOOOOO..",
    ".OOOhOOOOOOhOOO.",
    ".OOOOOOOOOOOOOO.",
    ".OOOooooooooOOO.",
    "..OOOooooooOOO..",
    "..OOoOooooOoOO..",
    "..hho......ohh..",
    "..hho......ohh..",
];

const GODMODE_3: &[&str] = &[
    "....hhhhhhhh....",
    "..hhOOOOOOOOhh..",
    "..hOOOOOOOOOOh..",
    "..OOOkOOkkOOOO..",
    "..OOOOOOOOOOOO..",
    ".OOOhOOOOOOhOOO.",
    ".OOOOOOOOOOOOOO.",
    ".OOOooooooooOOO.",
    "..OOOooooooOOO..",
    "..OOoOooooOoOO..",
    "..hho......ohh..",
    "..hho......ohh..",
];

const HURT_1: &[&str] = &[
    "................",
    "....OOOOOOOO....",
    "...OOOOOOOOOO...",
    "..OOOkdOOdkOO...",
    "..OOOOOOOOOOOO..",
    ".OOOdOOOOOOdOOO.",
    ".OOOddooodddOOO.",
    ".OOOOOOOOOOOOOO.",
    "..OOOddddddOOO..",
    "..OOdOooooOdOO..",
    "...dd......dd...",
    "...dd......dd...",
];

const HURT_2: &[&str] = &[
    "................",
    "....OOOOOOOO....",
    "...OOOOOOOOOO...",
    "..OOOddkkddOO...",
    "..OOOOOOOOOOOO..",
    ".OOOdOOOOOOdOOO.",
    ".OOOddooodddOOO.",
    ".OOOOOOOOOOOOOO.",
    "..OOOddddddOOO..",
    "..OOdOooooOdOO..",
    "...dd......dd...",
    "...dd......dd...",
];

const HURT_3: &[&str] = &[
    "................",
    "....OOOOOOOO....",
    "...OOOOOOOOOO...",
    "..OOOdkOOddOO...",
    "..OOOOOOOOOOOO..",
    ".OOOdOOOOOOdOOO.",
    ".OOOddooodddOOO.",
    ".OOOOOOOOOOOOOO.",
    "..OOOddddddOOO..",
    "..OOdOooooOdOO..",
    "...dd......dd...",
    "...dd......dd...",
];

const THINKING_1: &[&str] = &[
    ".......hh.......",
    "....OOOOOOOO....",
    "...OOOOOOOOOO...",
    "..OOOkOOkkOOO...",
    "..OOOOOOOOOOOO..",
    ".OOOhOOOOOOdOOO.",
    ".OOOooooooooOOO.",
    ".OOOOOOOOOOOOOO.",
    "..OOOddddddOOO..",
    "..OOdOooooOdOO..",
    "...oo......oo...",
    "...oo......oo...",
];

const THINKING_2: &[&str] = &[
    "......hhhh......",
    "....OOOOOOOO....",
    "...OOOOOOOOOO...",
    "..OOOOkkkkOOO...",
    "..OOOOOOOOOOOO..",
    ".OOOhOOOOOOdOOO.",
    ".OOOooooooooOOO.",
    ".OOOOOOOOOOOOOO.",
    "..OOOddddddOOO..",
    "..OOdOooooOdOO..",
    "...oo......oo...",
    "...oo......oo...",
];

const THINKING_3: &[&str] = &[
    ".......hh.......",
    "....OOOOOOOO....",
    "...OOOOOOOOOO...",
    "..OOOkOOOOkOO...",
    "..OOOOOOOOOOOO..",
    ".OOOhOOOOOOdOOO.",
    ".OOOooooooooOOO.",
    ".OOOOOOOOOOOOOO.",
    "..OOOddddddOOO..",
    "..OOdOooooOdOO..",
    "...oo......oo...",
    "...oo......oo...",
];

/// Build the embedded default face pack.
pub(crate) fn load_default_pack() -> Result<FacePack> {
    FacePack::new(vec![
        vec![frame(NEUTRAL_1), frame(NEUTRAL_2), frame(NEUTRAL_3)],
        vec![frame(HAPPY_1), frame(HAPPY_2), frame(HAPPY_3)],
        vec![frame(ANGRY_1), frame(ANGRY_2), frame(ANGRY_3)],
        vec![frame(GODMODE_1), frame(GODMODE_2), frame(GODMODE_3)],
        vec![frame(HURT_1), frame(HURT_2), frame(HURT_3)],
        vec![frame(THINKING_1), frame(THINKING_2), frame(THINKING_3)],
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
