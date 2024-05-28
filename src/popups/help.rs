use crossterm::event::{KeyCode, KeyEvent};
use pltx_app::{state::AppPopup, App, Popup};
use pltx_widgets::{PopupSize, PopupWidget};
use ratatui::{layout::Rect, Frame};

pub struct Help {
    size: PopupSize,
}

impl Popup for Help {
    fn init(_: &App) -> Help {
        Help {
            size: PopupSize::default().percentage_based_height().height(90),
        }
    }

    fn key_event_handler(&mut self, app: &mut App, key_event: KeyEvent) {
        if key_event.code == KeyCode::Char('?') {
            app.reset_display();
            app.popup = AppPopup::None;
        }
    }

    fn render(&self, app: &App, frame: &mut Frame, area: Rect) {
        PopupWidget::new(app, area)
            .title_top("Help Menu")
            .size(self.size.clone())
            .render(frame);
    }
}
