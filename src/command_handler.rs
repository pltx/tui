use crossterm::event::{KeyCode, KeyEvent};
use nucleo::{
    pattern::{Atom, AtomKind, CaseMatching, Normalization},
    Matcher,
};
use pltx_app::{
    state::{AppModule, AppPopup, Display},
    App, DefaultWidget, KeyEventHandler, Popup,
};
use pltx_widgets::{self, PopupSize, PopupWidget, TextInput};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Style, Stylize},
    text::{Line, Text},
    widgets::{Block, BorderType, Borders, Padding, Paragraph},
    Frame,
};

#[derive(PartialEq, Clone)]
enum Command {
    Dashboard,
    Help,
    ProjectManagement,
    Quit,
    None,
}

#[derive(PartialEq)]
enum Content {
    CommandInput,
    // Output,
}

#[derive(PartialEq)]
enum FocusedPane {
    Input,
    Options,
}
pub struct CommandHandler {
    command: TextInput,
    size: PopupSize,
    content: Content,
    focused_pane: FocusedPane,
    command_options: Vec<String>,
    selected_option: usize,
    matcher: Matcher,
}

fn command_data<'a>() -> Vec<(Command, &'a str)> {
    fn get_command<'b>(cmd: Command) -> &'b str {
        match cmd {
            Command::Dashboard => "dashboard",
            Command::Help => "help",
            Command::ProjectManagement => "project management",
            Command::Quit => "quit",
            Command::None => "",
        }
    }

    let cmds = [
        Command::Dashboard,
        Command::Help,
        Command::ProjectManagement,
        Command::Quit,
    ];

    let mut list = vec![];
    for cmd in cmds {
        list.push((cmd.clone(), get_command(cmd)))
    }

    list
}

fn command_strings<'a>() -> Vec<&'a str> {
    command_data().iter().map(|c| c.1).collect::<Vec<&str>>()
}

impl Popup for CommandHandler {
    fn init(_: &App) -> CommandHandler {
        let size = PopupSize::default().width(60).height(20);

        CommandHandler {
            command: TextInput::new("Command")
                .display(Display::command())
                .size((size.width - 2, size.height - 2))
                .placeholder("Enter a command...")
                .max(50),
            size,
            content: Content::CommandInput,
            focused_pane: FocusedPane::Input,
            command_options: command_strings().iter().map(|s| s.to_string()).collect(),
            selected_option: 0,
            matcher: nucleo::Matcher::default(),
        }
    }

    fn key_event_handler(&mut self, app: &mut App, key_event: KeyEvent) {
        if self.focused_pane == FocusedPane::Input {
            self.command.key_event_handler(app, key_event);
            self.update_options();
        }

        if app.is_normal_mode() {
            match key_event.code {
                KeyCode::Enter => self.execute_command(app),
                KeyCode::Char('q') => {
                    app.reset_display();
                    self.reset();
                }
                KeyCode::Char('j') => {
                    if self.content == Content::CommandInput
                        && self.focused_pane == FocusedPane::Input
                    {
                        self.focused_pane = FocusedPane::Options;
                    }
                }
                KeyCode::Char('k') => {
                    if self.content == Content::CommandInput
                        && self.focused_pane == FocusedPane::Options
                    {
                        self.focused_pane = FocusedPane::Input;
                    }
                }
                _ => {}
            }
        } else if app.is_insert_mode() {
            match key_event.code {
                KeyCode::Enter => self.execute_command(app),
                KeyCode::Esc => app.command_display(),
                _ => {}
            }
        }
    }

    fn render(&self, app: &App, frame: &mut Frame, area: Rect) {
        let colors = &app.config.colors;

        let popup = PopupWidget::new(app, area)
            .size(self.size.clone())
            .render(frame);

        let [input_layout, command_list_layout] = Layout::default()
            .vertical_margin(1)
            .horizontal_margin(2)
            .constraints([Constraint::Length(3), Constraint::Min(1)])
            .areas(popup.popup_area);

        self.command.render(
            frame,
            app,
            input_layout,
            self.focused_pane == FocusedPane::Input,
        );

        let text = if self.command_options.is_empty() {
            Text::from("No commands found.")
        } else {
            Text::from(
                self.command_options
                    .iter()
                    .enumerate()
                    .map(|(i, o)| {
                        Line::from(format!(" {o} ")).style(if i == self.selected_option {
                            Style::new()
                                .bold()
                                .fg(colors.active_fg)
                                .bg(colors.active_bg)
                        } else {
                            Style::new().fg(colors.secondary_fg)
                        })
                    })
                    .collect::<Vec<Line>>(),
            )
        };

        let command_list = Paragraph::new(text).block(
            Block::new()
                .padding(Padding::horizontal(1))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(
                    Style::new().fg(if self.focused_pane == FocusedPane::Options {
                        colors.primary
                    } else {
                        colors.border
                    }),
                ),
        );

        frame.render_widget(command_list, command_list_layout);
    }
}

impl CommandHandler {
    fn reset(&mut self) {
        self.command.reset();
        self.update_options();
        self.focused_pane = FocusedPane::Input;
    }

    fn parse_command(&self) -> Command {
        if self.command_options.is_empty() {
            return Command::None;
        }
        let command_str = self.command_options[self.selected_option].clone();
        for command in command_data() {
            if command.1.contains(&command_str) {
                return command.0;
            }
        }
        Command::None
    }

    fn execute_command(&mut self, app: &mut App) {
        let command = self.parse_command();
        match command {
            Command::Dashboard => {
                app.reset_display();
                app.module = AppModule::Dashboard;
            }
            Command::Help => {
                app.popup_display();
                app.popup = AppPopup::Help;
            }
            Command::ProjectManagement => {
                app.reset_display();
                app.module = AppModule::ProjectManagement;
            }
            Command::Quit => app.exit(),
            Command::None => {}
        }
        if command != Command::None {
            self.reset()
        }
    }

    fn update_options(&mut self) {
        let is_longer_than_longest_option = self.command.input_string().chars().count()
            > command_data()
                .iter()
                .map(|c| c.1.chars().count())
                .max()
                .unwrap_or(0);
        if is_longer_than_longest_option {
            self.command_options = vec![];
        } else if self.command.input_string().chars().count() == 0 {
            let command_list = command_data().iter().map(|c| c.1).collect::<Vec<&str>>();
            self.command_options = command_list.iter().map(|s| s.to_string()).collect();
        } else {
            let pattern = Atom::new(
                &self.command.input_string(),
                CaseMatching::Smart,
                Normalization::Smart,
                AtomKind::Fuzzy,
                false,
            );
            self.command_options = pattern
                .match_list(command_strings(), &mut self.matcher)
                .iter()
                .map(|s| s.0.to_string())
                .collect::<Vec<String>>();
        }
        self.selected_option = 0;
    }
}
