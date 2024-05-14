use std::str::FromStr;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Padding, Paragraph, Widget},
    Frame,
};

use super::{projects::ProjectsState, screen::ScreenPane};
use crate::{
    components::{TextInput, TextInputEvent},
    config::ColorsConfig,
    state::{Mode, State},
    trace_debug,
    utils::{Init, KeyEventHandler, RenderPage},
    App,
};

const PROJECT_TITLE_MAX_LENGTH: usize = 100;
const PROJECT_DESCRIPTION_MAX_LENGTH: usize = 500;
const LABEL_TITLE_MAX_LENTH: usize = 15;
const LABEL_COLOR_REQUIRED_LENTH: usize = 7;

#[derive(PartialEq)]
enum FocusedPane {
    Title,
    Description,
    Labels,
    Actions,
}

#[derive(PartialEq)]
enum Action {
    Save,
    Cancel,
}

struct Inputs {
    title: TextInput,
    description: TextInput,
    labels: Vec<(Option<i32>, TextInput, TextInput)>,
}

#[derive(Clone)]
pub struct ProjectLabel {
    pub project_id: i32,
    pub id: i32,
    pub title: String,
    pub color: String,
}

#[derive(Clone)]
struct ProjectData {
    id: i32,
    title: String,
    description: Option<String>,
    labels: Vec<ProjectLabel>,
}

#[derive(PartialEq)]
enum LabelOption {
    Labels,
    AddLabel,
}

#[derive(PartialEq)]
enum LabelCol {
    Title,
    Color,
}

pub struct ProjectEditor {
    new: bool,
    data: Option<ProjectData>,
    focused_pane: FocusedPane,
    action: Action,
    inputs: Inputs,
    selected_label: usize,
    focused_label_option: LabelOption,
    label_col: LabelCol,
}

impl Init for ProjectEditor {
    fn init(_: &mut App) -> ProjectEditor {
        ProjectEditor {
            new: false,
            data: None,
            focused_pane: FocusedPane::Title,
            action: Action::Save,
            inputs: Inputs {
                title: TextInput::new(Mode::Navigation)
                    .title("Title")
                    .max(PROJECT_TITLE_MAX_LENGTH),
                description: TextInput::new(Mode::Navigation)
                    .title("Description")
                    .max(PROJECT_DESCRIPTION_MAX_LENGTH),
                labels: vec![],
            },
            selected_label: 0,
            focused_label_option: LabelOption::Labels,
            label_col: LabelCol::Title,
        }
    }
}

impl ProjectEditor {
    fn db_new_project(&self, app: &App) -> rusqlite::Result<()> {
        let description = if self.inputs.description.input_string().chars().count() == 0 {
            None
        } else {
            Some(self.inputs.description.input_string())
        };

        let highest_position = app.db.get_highest_position("project").unwrap();
        app.db.conn.execute(
            "INSERT INTO project (title, description, position) VALUES (?1, ?2, ?3)",
            (
                self.inputs.title.input_string(),
                description,
                highest_position.saturating_add(1),
            ),
        )?;

        let new_project_id = app.db.last_row_id("project")?;

        self.db_new_labels(app, new_project_id)?;

        Ok(())
    }

    fn db_new_labels(&self, app: &App, project_id: i32) -> rusqlite::Result<()> {
        for (i, label) in self.inputs.labels.iter().enumerate() {
            let query = "INSERT INTO project_label (project_id, title, color, position) VALUES \
                         (?1, ?2, ?3, ?4)";
            app.db.conn.execute(
                query,
                (
                    project_id,
                    label.1.input_string(),
                    label.2.input_string(),
                    i,
                ),
            )?;
        }

        Ok(())
    }

    fn db_edit_project(&self, app: &App) -> rusqlite::Result<()> {
        if let Some(data) = &self.data {
            let query = "UPDATE project SET title = ?1, description = ?2 WHERE id = ?3";
            let mut stmt = app.db.conn.prepare(query)?;
            stmt.execute((
                self.inputs.title.input_string(),
                self.inputs.description.input_string(),
                data.id,
            ))?;
            self.db_edit_labels(app, data.id)?;
        } else {
            panic!("project data was not set")
        }

        Ok(())
    }

    fn db_edit_labels(&self, app: &App, project_id: i32) -> rusqlite::Result<()> {
        trace_debug!(self.inputs.labels.len());
        for (i, label) in self.inputs.labels.iter().enumerate() {
            if let Some(label_id) = label.0 {
                let query = "UPDATE project_label SET title = ?1, color = ?2 WHERE project_id = \
                             ?3 and id = ?4";
                let mut stmt = app.db.conn.prepare(query)?;
                stmt.execute((
                    label.1.input_string(),
                    label.2.input_string(),
                    project_id,
                    label_id,
                ))?;
            } else {
                let query = "INSERT INTO project_label (project_id, title, color, position) \
                             VALUES (?1, ?2, ?3, ?4)";
                app.db.conn.execute(
                    query,
                    (
                        project_id,
                        label.1.input_string(),
                        label.2.input_string(),
                        i,
                    ),
                )?;
            }
        }

        Ok(())
    }
}

impl KeyEventHandler<bool> for ProjectEditor {
    fn key_event_handler(&mut self, app: &mut App, key_event: KeyEvent, _: &State) -> bool {
        match self.focused_pane {
            FocusedPane::Title => self.inputs.title.handle_key_event(app, key_event),
            FocusedPane::Description => self.inputs.description.handle_key_event(app, key_event),
            FocusedPane::Labels => {
                if !self.inputs.labels.is_empty() {
                    if self.label_col == LabelCol::Title {
                        self.inputs.labels[self.selected_label]
                            .1
                            .handle_key_event(app, key_event)
                    } else {
                        self.inputs.labels[self.selected_label]
                            .2
                            .handle_key_event(app, key_event)
                    };
                    TextInputEvent::None
                } else {
                    TextInputEvent::None
                }
            }
            _ => TextInputEvent::None,
        };

        if app.state.mode == Mode::Navigation {
            match key_event.code {
                KeyCode::Char('n') => app.state.mode = Mode::Insert,
                KeyCode::Char('[') => {
                    self.reset();
                    return true;
                }
                KeyCode::BackTab => self.prev_label(),
                KeyCode::Tab => self.next_label(),
                KeyCode::Char('j') => self.next_focus(),
                KeyCode::Char('k') => self.prev_focus(),
                KeyCode::Enter => {
                    if self.focused_pane == FocusedPane::Labels
                        && self.focused_label_option == LabelOption::AddLabel
                    {
                        self.add_label(app);
                    } else if self.save_project(app) {
                        return true;
                    }
                }
                _ => {}
            }
        }

        if app.state.mode == Mode::Insert && key_event.code == KeyCode::Tab {
            self.next_label();
        }

        false
    }
}

impl ProjectEditor {
    fn next_focus(&mut self) {
        match self.focused_pane {
            FocusedPane::Title => self.focused_pane = FocusedPane::Description,
            FocusedPane::Description => {
                self.focused_pane = FocusedPane::Labels;
                self.focused_label_option = if self.inputs.labels.is_empty() {
                    LabelOption::AddLabel
                } else {
                    LabelOption::Labels
                };
            }
            FocusedPane::Labels => {
                if self.focused_label_option == LabelOption::Labels {
                    self.focused_label_option = LabelOption::AddLabel;
                } else {
                    self.focused_pane = FocusedPane::Actions;
                }
            }
            FocusedPane::Actions => {
                if self.action == Action::Save {
                    self.action = Action::Cancel;
                } else if self.action == Action::Cancel {
                    self.focused_pane = FocusedPane::Title;
                    self.action = Action::Save;
                }
            }
        }
    }

    fn prev_focus(&mut self) {
        match self.focused_pane {
            FocusedPane::Title => {
                self.focused_pane = FocusedPane::Actions;
                self.action = Action::Cancel;
            }
            FocusedPane::Description => self.focused_pane = FocusedPane::Title,
            FocusedPane::Labels => {
                if self.focused_label_option == LabelOption::AddLabel
                    && !self.inputs.labels.is_empty()
                {
                    self.focused_label_option = LabelOption::Labels;
                } else {
                    self.focused_pane = FocusedPane::Description;
                }
            }
            FocusedPane::Actions => {
                if self.action == Action::Save {
                    self.focused_pane = FocusedPane::Labels;
                    self.focused_label_option = LabelOption::AddLabel;
                } else if self.action == Action::Cancel {
                    self.action = Action::Save;
                }
            }
        }
    }

    fn next_label(&mut self) {
        if self.focused_pane == FocusedPane::Labels
            && self.focused_label_option == LabelOption::Labels
        {
            if self.label_col == LabelCol::Color {
                if self.inputs.labels.len().saturating_sub(1) == self.selected_label {
                    self.selected_label = 0;
                } else {
                    self.selected_label = self.selected_label.saturating_add(1);
                }
                self.label_col = LabelCol::Title;
            } else {
                self.label_col = LabelCol::Color;
            }
        }
    }

    fn prev_label(&mut self) {
        if self.focused_pane == FocusedPane::Labels
            && self.focused_label_option == LabelOption::Labels
        {
            if self.label_col == LabelCol::Title {
                if self.selected_label == 0 {
                    self.selected_label = self.inputs.labels.len().saturating_sub(1);
                } else {
                    self.selected_label = self.selected_label.saturating_sub(1);
                }
                self.label_col = LabelCol::Color;
            } else {
                self.label_col = LabelCol::Title;
            }
        }
    }

    fn add_label(&mut self, app: &mut App) {
        let title_input = TextInput::new(Mode::Navigation)
            .placeholder("Title")
            .required()
            .max(LABEL_TITLE_MAX_LENTH);
        let mut color_input = TextInput::new(Mode::Navigation)
            .placeholder("Color")
            .required_len(LABEL_COLOR_REQUIRED_LENTH);
        color_input.input(app.config.colors.fg.to_string());
        self.inputs.labels.push((None, title_input, color_input));
        self.selected_label = self.inputs.labels.len().saturating_sub(1);
        self.focused_label_option = LabelOption::Labels;
        self.label_col = LabelCol::Title;
        app.state.mode = Mode::Insert;
    }

    fn save_project(&mut self, app: &mut App) -> bool {
        if self.focused_pane == FocusedPane::Actions {
            if self.action == Action::Save {
                if self.new {
                    self.db_new_project(app).unwrap_or_else(|e| panic!("{e}"));
                } else {
                    self.db_edit_project(app).unwrap_or_else(|e| panic!("{e}"));
                }
                self.reset()
            } else if self.action == Action::Cancel {
                self.reset()
            }
            return true;
        }
        false
    }
}

impl RenderPage<ProjectsState> for ProjectEditor {
    fn render(&mut self, app: &mut App, frame: &mut Frame, area: Rect, state: ProjectsState) {
        let colors = &app.config.colors.clone();
        let main_sp = state.screen_pane == ScreenPane::Main;

        let block = Block::new()
            .title(if self.new {
                " New Project "
            } else {
                " Edit Project "
            })
            .padding(Padding::horizontal(1))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::new().fg(colors.border));
        frame.render_widget(block, area);

        let border_height = 2;
        let new_label_height = 1;

        let [title_layout, description_layout, label_layout, actions_layout] = Layout::default()
            .vertical_margin(1)
            .horizontal_margin(2)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(5),
                Constraint::Length(
                    self.inputs.labels.len() as u16 + border_height + new_label_height,
                ),
                Constraint::Length(4),
            ])
            .areas(area);

        let [fixed_width_label_layout] = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(60)])
            .areas(label_layout);

        frame.render_widget(self.render_title(app, title_layout, main_sp), title_layout);

        frame.render_widget(
            self.render_description(app, description_layout, main_sp),
            description_layout,
        );

        let labels = self.render_labels(app, fixed_width_label_layout, main_sp);
        frame.render_widget(labels.0, fixed_width_label_layout);
        for label_widget in labels.1 {
            frame.render_widget(label_widget.0 .0, label_widget.0 .1);
            frame.render_widget(label_widget.1 .0, label_widget.1 .1);
            frame.render_widget(label_widget.2 .0, label_widget.2 .1);
        }
        frame.render_widget(labels.2 .0, labels.2 .1);

        let actions = self.render_actions(colors, actions_layout, main_sp);
        frame.render_widget(actions.0, actions.1);
    }
}

impl ProjectEditor {
    fn render_title(&self, app: &mut App, area: Rect, main_sp: bool) -> impl Widget {
        let focused = self.focused_pane == FocusedPane::Title && main_sp;
        self.inputs
            .title
            .render_block(app, area.width - 2, area.height - 2, focused)
    }

    fn render_description(&self, app: &mut App, area: Rect, main_sp: bool) -> impl Widget {
        let focused = self.focused_pane == FocusedPane::Description && main_sp;
        self.inputs
            .description
            .render_block(app, area.width - 2, area.height - 2, focused)
    }

    fn render_actions(
        &self,
        colors: &ColorsConfig,
        area: Rect,
        main_sp: bool,
    ) -> (impl Widget, Rect) {
        let width = 30;
        let [layout] = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(width)])
            .areas(area);

        (
            Paragraph::new(Text::from(
                [
                    (
                        Action::Save,
                        if self.new {
                            "Create New Project"
                        } else {
                            "Save Project"
                        },
                    ),
                    (Action::Cancel, "Cancel"),
                ]
                .iter()
                .map(|l| {
                    Line::from(vec![
                        Span::from(format!(" {} ", l.1)),
                        Span::from(" ".repeat(layout.width as usize - 2 - l.1.chars().count() - 2)),
                    ])
                    .style(if self.focused_pane == FocusedPane::Actions {
                        if self.action == l.0 {
                            Style::new()
                                .bold()
                                .fg(colors.active_fg)
                                .bg(colors.active_bg)
                        } else {
                            Style::new().fg(colors.secondary)
                        }
                    } else {
                        Style::new().fg(colors.secondary)
                    })
                })
                .collect::<Vec<Line>>(),
            ))
            .block(
                Block::new()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::new().fg(
                        if self.focused_pane == FocusedPane::Actions && main_sp {
                            colors.primary
                        } else {
                            colors.border
                        },
                    )),
            ),
            layout,
        )
    }

    #[allow(clippy::type_complexity)]
    fn render_labels(
        &self,
        app: &App,
        area: Rect,
        main_sp: bool,
    ) -> (
        impl Widget,
        Vec<(
            (impl Widget, Rect),
            (impl Widget, Rect),
            (impl Widget, Rect),
        )>,
        (impl Widget, Rect),
    ) {
        let colors = &app.config.colors;

        let mut label_widgets = vec![];

        let [label_list_layout, add_label_layout] = Layout::default()
            .margin(1)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .areas(area);

        let split_label_list_layout = Layout::default()
            .constraints(
                self.inputs
                    .labels
                    .iter()
                    .map(|_| Constraint::Length(1))
                    .collect::<Vec<Constraint>>(),
            )
            .split(label_list_layout);

        let block = Block::new()
            .padding(Padding::horizontal(1))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(
                if self.focused_pane == FocusedPane::Labels && main_sp {
                    colors.primary
                } else {
                    colors.border
                },
            ));

        for (i, label_input) in self.inputs.labels.iter().enumerate() {
            let [title_layout, color_layout, preview_layout] = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Fill(1),
                    Constraint::Fill(1),
                    Constraint::Fill(1),
                ])
                .areas(split_label_list_layout[i]);

            let is_focused = self.focused_pane == FocusedPane::Labels
                && self.focused_label_option == LabelOption::Labels
                && self.selected_label == i;

            let label_title_input = label_input.1.render_text(
                app,
                title_layout.width,
                title_layout.height,
                is_focused && self.label_col == LabelCol::Title,
            );

            let label_color_input = label_input.2.render_text(
                app,
                color_layout.width,
                color_layout.height,
                is_focused && self.label_col == LabelCol::Color,
            );

            let label_preview_input = Paragraph::new(format!(" {} ", label_input.1.input_string()))
                .fg(Color::from_str(&label_input.2.input_string()).unwrap_or(colors.bg));

            label_widgets.push((
                (label_title_input, title_layout),
                (label_color_input, color_layout),
                (label_preview_input, preview_layout),
            ))
        }

        let add_label = Line::from(" Add Label ").style(
            if self.focused_pane == FocusedPane::Labels
                && self.focused_label_option == LabelOption::AddLabel
            {
                Style::new()
                    .bold()
                    .fg(colors.active_fg)
                    .bg(colors.active_bg)
            } else {
                Style::new()
            },
        );

        (block, label_widgets, (add_label, add_label_layout))
    }

    pub fn set_new(mut self) -> Self {
        self.new = true;
        self
    }

    pub fn set_project(&mut self, app: &App, project_id: i32) -> rusqlite::Result<()> {
        let project_query = "SELECT id, title, description FROM project WHERE id = ?1";
        let mut project_stmt = app.db.conn.prepare(project_query)?;
        let mut project = project_stmt.query_row([project_id], |r| {
            Ok(ProjectData {
                id: r.get(0)?,
                title: r.get(1)?,
                description: r.get(2)?,
                labels: vec![],
            })
        })?;

        project.labels = self.db_get_labels(app, project_id)?;

        self.data = Some(project.clone());

        self.inputs.title.input(project.title);
        self.inputs
            .description
            .input(if let Some(desc) = project.description {
                desc
            } else {
                String::from("")
            });

        Ok(())
    }

    fn db_get_labels(&mut self, app: &App, project_id: i32) -> rusqlite::Result<Vec<ProjectLabel>> {
        let query = "SELECT project_id, id, title, color FROM project_label WHERE project_id = ?1";
        let mut stmt = app.db.conn.prepare(query)?;
        let labels_iter = stmt.query_map([project_id], |r| {
            Ok(ProjectLabel {
                project_id: r.get(0)?,
                id: r.get(1)?,
                title: r.get(2)?,
                color: r.get(3)?,
            })
        })?;

        let mut labels = vec![];
        for l in labels_iter {
            let label = l.unwrap();

            let mut title_input = TextInput::new(Mode::Navigation)
                .placeholder("Title")
                .required()
                .max(LABEL_TITLE_MAX_LENTH);
            title_input.input(label.title.clone());
            let mut color_input = TextInput::new(Mode::Navigation)
                .placeholder("Color")
                .max(LABEL_COLOR_REQUIRED_LENTH);
            color_input.input(label.color.clone());

            let label_position = self
                .inputs
                .labels
                .iter()
                .position(|p| p.0.is_some_and(|id| id == label.id));

            if let Some(pos) = label_position {
                self.inputs.labels[pos].1 = title_input;
                self.inputs.labels[pos].2 = color_input;
            } else {
                self.inputs
                    .labels
                    .push((Some(label.id), title_input, color_input))
            }

            labels.push(label);
        }

        if !labels.is_empty() {
            self.selected_label = 0;
        }

        Ok(labels)
    }

    fn reset(&mut self) {
        self.focused_pane = FocusedPane::Title;
        self.action = Action::Save;
        self.inputs.title.reset();
        self.inputs.description.reset();
    }
}
