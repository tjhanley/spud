use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Text},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::{graphics::GraphicsBackend, layout::DoomRects};

/// Raw RGBA frame data for the agent face, passed into [`ShellView`].
pub struct FaceFrame<'a> {
    /// Raw RGBA pixel data (row-major, 4 bytes/pixel).
    pub data: &'a [u8],
    /// Pixel width of the frame.
    pub width: u32,
    /// Pixel height of the frame.
    pub height: u32,
}

/// Data passed to [`render_shell`] to populate the shell chrome.
///
/// The shell view carries the text content for the top bar, HUD panels, and
/// delegates hero-area rendering to the caller via a closure.
pub struct ShellView<'a> {
    /// Title of the active module, shown in the top bar.
    pub module_title: &'a str,
    /// Status text displayed alongside the module title.
    pub status_line: &'a str,
    /// Lines rendered in the left HUD column.
    pub hud_left: Vec<String>,
    /// Lines rendered in the right HUD column.
    pub hud_right: Vec<String>,
    /// Agent face frame to render in the HUD centre panel.
    pub face: Option<FaceFrame<'a>>,
    /// Graphics backend to use for face rendering.
    pub graphics_backend: GraphicsBackend,
}

/// Render the full Doom-style shell: top bar, hero area, and HUD panels.
///
/// The `hero` closure is called to let the active module draw into the hero
/// rectangle. All other chrome (top bar, HUD left/right, agent face) is
/// rendered by this function.
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

    f.render_widget(
        Block::default().borders(Borders::ALL).title("HUD"),
        rects.hud,
    );

    let left_text = Text::from(
        view.hud_left
            .into_iter()
            .map(Line::from)
            .collect::<Vec<_>>(),
    );
    let left =
        Paragraph::new(left_text).block(Block::default().borders(Borders::ALL).title("LEFT"));
    f.render_widget(left, rects.hud_left);

    // Face rendering depends on graphics backend
    match view.graphics_backend {
        GraphicsBackend::Sixel => {
            // For Sixel, we just draw the border and clear the inner area.
            // The actual image will be written post-draw via terminal.backend_mut().
            let face_block = Block::default().borders(Borders::ALL).title("AGENT");
            let inner = face_block.inner(rects.hud_face);
            f.render_widget(face_block, rects.hud_face);

            // Clear inner area to give ratatui stable content (prevents the diff
            // algorithm from overwriting our image). Clear is more efficient than
            // manually filling with spaces.
            if view.face.is_some() {
                f.render_widget(Clear, inner);
            }
        }
        GraphicsBackend::UnicodeBlock => {
            // Unicode half-block rendering
            if let Some(face) = view.face {
                crate::face::render_face(
                    f.buffer_mut(),
                    rects.hud_face,
                    face.data,
                    face.width,
                    face.height,
                );
            } else {
                let face = Paragraph::new(Line::from("[ FACE ]"))
                    .block(Block::default().borders(Borders::ALL).title("AGENT"));
                f.render_widget(face, rects.hud_face);
            }
        }
        _ => {
            // Future backends (Kitty, ITerm2) — fall back to placeholder
            let face = Paragraph::new(Line::from("[ FACE ]"))
                .block(Block::default().borders(Borders::ALL).title("AGENT"));
            f.render_widget(face, rects.hud_face);
        }
    }

    let right_text = Text::from(
        view.hud_right
            .into_iter()
            .map(Line::from)
            .collect::<Vec<_>>(),
    );
    let right =
        Paragraph::new(right_text).block(Block::default().borders(Borders::ALL).title("RIGHT"));
    f.render_widget(right, rects.hud_right);
}
