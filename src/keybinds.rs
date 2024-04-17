use color_eyre::eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};

use crate::{
    state::{Mode, Popup, State, Window},
    App,
};

// Makes keybinds by handing key events
pub fn handle_key_event(app: &mut App, key_event: KeyEvent, event_state: &State) -> Result<()> {
    let screen_index = app
        .screen_list
        .iter()
        .position(|s| s.0 == event_state.screen)
        .unwrap();

    if event_state.mode == Mode::Navigation {
        match key_event.code {
            // Show the help menu
            KeyCode::Char('?') => {
                app.state.mode = Mode::Popup;
                app.state.popup = Popup::Help;
            }
            // Quit the application
            KeyCode::Char('q') | KeyCode::Char('Q') => app.exit(),
            // Go down an option
            KeyCode::Char('j') => {
                if screen_index == app.screen_list.len() - 1 {
                    app.state.screen = app.screen_list[0].0.clone();
                } else {
                    app.state.screen = app.screen_list[screen_index + 1].0.clone();
                }
            }
            // Go up an option
            KeyCode::Char('k') => {
                if screen_index == 0 {
                    app.state.screen = app.screen_list[app.screen_list.len() - 1].0.clone();
                } else {
                    app.state.screen = app.screen_list[screen_index - 1].0.clone();
                }
            }
            // Focus on the previous window
            KeyCode::Char('h') => {
                if event_state.window == Window::Screen {
                    app.state.window = Window::Navigation;
                }
            }
            // Focus on the next window
            KeyCode::Char('l') => {
                if event_state.window == Window::Navigation {
                    app.state.window = Window::Screen;
                }
            }
            _ => {}
        }
    }

    if event_state.mode == Mode::Popup {
        // Global popup keybinds
        match key_event.code {
            // Close the popup
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                app.state.mode = Mode::Navigation;
                app.state.popup = Popup::None;
            }
            _ => {}
        }
    }
    Ok(())
}
