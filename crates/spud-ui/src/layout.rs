use ratatui::layout::{Constraint, Direction, Layout, Rect};

#[derive(Debug, Clone, Copy)]
pub struct DoomRects {
    pub top: Rect,
    pub hero: Rect,
    pub hud: Rect,
    pub hud_left: Rect,
    pub hud_face: Rect,
    pub hud_right: Rect,
}

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
