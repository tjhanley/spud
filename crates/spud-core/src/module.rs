use crate::event::Event;

#[derive(Default)]
pub struct HudContribution {
    pub left_lines: Vec<String>,
    pub right_lines: Vec<String>,
}

pub trait Module {
    fn id(&self) -> &'static str;
    fn title(&self) -> &'static str;

    fn handle_event(&mut self, _ev: &Event) {}

    fn hud(&self) -> HudContribution {
        HudContribution::default()
    }
}
