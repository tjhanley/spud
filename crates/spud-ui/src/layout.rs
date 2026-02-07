use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// Pre-computed rectangles for the Doom-style screen layout.
///
/// The layout divides the terminal into three vertical bands:
/// 1. **Top bar** — a single-line status/title row.
/// 2. **Hero area** — the main content area rendered by the active module.
/// 3. **HUD** — a bottom panel split into left stats, a centre face, and right stats.
#[derive(Debug, Clone, Copy)]
pub struct DoomRects {
    /// The single-line top bar area.
    pub top: Rect,
    /// The main content area where the active module renders.
    pub hero: Rect,
    /// The full HUD band at the bottom of the screen.
    pub hud: Rect,
    /// Left column of the HUD (e.g. keybindings, status).
    pub hud_left: Rect,
    /// Centre column of the HUD (agent face).
    pub hud_face: Rect,
    /// Right column of the HUD (e.g. metrics).
    pub hud_right: Rect,
}

/// Compute the Doom-style layout rectangles for the given terminal area.
///
/// `hud_height` controls how many rows the bottom HUD occupies (clamped to a
/// minimum of 5). `face_width` sets the width of the centre face column.
pub fn doom_layout(area: Rect, hud_height: u16, face_width: u16) -> DoomRects {
    let hud_height = hud_height.max(5).min(area.height.saturating_sub(2).max(5));
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),          // top bar
            Constraint::Min(1),             // hero
            Constraint::Length(hud_height), // hud
        ])
        .split(area);

    let hud = chunks[2];
    let face_width = face_width.min(hud.width.saturating_sub(2).max(10));

    let hud_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(45),
            Constraint::Length(face_width),
            Constraint::Percentage(45),
        ])
        .split(hud);

    DoomRects {
        top: chunks[0],
        hero: chunks[1],
        hud,
        hud_left: hud_cols[0],
        hud_face: hud_cols[1],
        hud_right: hud_cols[2],
    }
}
