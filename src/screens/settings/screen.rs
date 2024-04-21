use ratatui::{layout::Rect, widgets::Paragraph, Frame};

use crate::{
    utils::{get_version, Init, RenderScreen},
    App,
};

pub struct Settings;

impl Init for Settings {
    fn init(_: &mut App) -> Settings {
        Settings {}
    }
}

impl RenderScreen for Settings {
    fn render(&mut self, _: &mut App, frame: &mut Frame, area: Rect) {
        let text = Paragraph::new(format!("Version: {}", get_version()));
        frame.render_widget(text, area);
    }
}