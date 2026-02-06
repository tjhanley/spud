use ratatui::{
    Frame,
    layout::Rect,
    widgets::{Block, Borders, Paragraph},
    style::Style,
    text::{Line, Text},
};

use crate::layout::DoomRects;

pub struct ShellView<'a> {
    pub module_title: &'a str,
    pub status_line: &'a str,
    pub hud_left: Vec<String>,
    pub hud_right: Vec<String>,
}

pub fn render_shell(
    f: &mut Frame,
    rects: DoomRects,
    view: ShellView<'_>,
    hero: impl FnOnce(&mut Frame, Rect),
) {
    let top = Paragraph::new(Line::from(format!(
        "SPUD | {} | {}",
        view.module_title, view.status_line
    )))
    .style(Style::default())
    .block(Block::default().borders(Borders::BOTTOM));
    f.render_widget(top, rects.top);

    hero(f, rects.hero);

    f.render_widget(Block::default().borders(Borders::ALL).title("HUD"), rects.hud);

    let left_text = Text::from(view.hud_left.into_iter().map(Line::from).collect::<Vec<_>>());
    let left = Paragraph::new(left_text).block(Block::default().borders(Borders::ALL).title("LEFT"));
    f.render_widget(left, rects.hud_left);

    let face = Paragraph::new(Line::from("[ FACE ]"))
        .block(Block::default().borders(Borders::ALL).title("AGENT"));
    f.render_widget(face, rects.hud_face);

    let right_text = Text::from(view.hud_right.into_iter().map(Line::from).collect::<Vec<_>>());
    let right = Paragraph::new(right_text).block(Block::default().borders(Borders::ALL).title("RIGHT"));
    f.render_widget(right, rects.hud_right);
}
