use std::str::FromStr;

use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use pltx_app::{App, CompositeWidget, DefaultWidget, KeyEventHandler, Screen};
use pltx_database::Database;
use pltx_utils::DateTime;
use pltx_widgets::{Buttons, TextInput};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::Line,
    widgets::{Block, BorderType, Borders, Padding, Paragraph},
    Frame,
};

const PROJECT_TITLE_MAX_LENGTH: usize = 50;
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

#[derive(PartialEq, Clone, Copy)]
enum Action {
    Save,
    Cancel,
}

struct Inputs {
    title: TextInput,
    description: TextInput,
    labels: Vec<(Option<i32>, TextInput, TextInput)>,
    actions: Buttons<Action>,
}

#[derive(Clone)]
pub struct ProjectLabel {
    pub project_id: i32,
    pub id: i32,
    pub title: String,
    pub color: String,
}

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
    inputs: Inputs,
    selected_label: usize,
    focused_label_option: LabelOption,
    label_col: LabelCol,
}

impl Screen<Result<bool>> for ProjectEditor {
    fn init(_: &App) -> Result<ProjectEditor> {
        Ok(ProjectEditor {
            new: false,
            data: None,
            focused_pane: FocusedPane::Title,
            inputs: Inputs {
                title: TextInput::new("Title").max(PROJECT_TITLE_MAX_LENGTH),
                description: TextInput::new("Description").max(PROJECT_DESCRIPTION_MAX_LENGTH),
                labels: vec![],
                actions: Buttons::from([
                    (Action::Save, "Save Project"),
                    (Action::Cancel, "Cancel"),
                ]),
            },
            selected_label: 0,
            focused_label_option: LabelOption::Labels,
            label_col: LabelCol::Title,
        })
    }

    fn key_event_handler(&mut self, app: &mut App, key_event: KeyEvent) -> Result<bool> {
        match self.focused_pane {
            FocusedPane::Title => self.inputs.title.key_event_handler(app, key_event),
            FocusedPane::Description => self.inputs.description.key_event_handler(app, key_event),
            FocusedPane::Labels => {
                if !self.inputs.labels.is_empty() {
                    if self.label_col == LabelCol::Title {
                        self.inputs.labels[self.selected_label]
                            .1
                            .key_event_handler(app, key_event)
                    } else {
                        self.inputs.labels[self.selected_label]
                            .2
                            .key_event_handler(app, key_event)
                    };
                }
            }
            _ => {}
        };

        if app.is_normal_mode() {
            match key_event.code {
                KeyCode::Char('[') => {
                    self.reset();
                    return Ok(true);
                }
                KeyCode::Char('K') => self.focus_prev(),
                KeyCode::Char('J') => self.focus_next(),
                KeyCode::Char('j') => match self.focused_pane {
                    FocusedPane::Labels => self.next_label(),
                    FocusedPane::Actions => self.inputs.actions.focus_next(),
                    _ => {}
                },
                KeyCode::Char('k') => match self.focused_pane {
                    FocusedPane::Labels => self.prev_label(),
                    FocusedPane::Actions => self.inputs.actions.focus_prev(),
                    _ => {}
                },
                KeyCode::Tab | KeyCode::BackTab => self.toggle_label_col(),
                KeyCode::Enter => {
                    if self.focused_pane == FocusedPane::Labels
                        && self.focused_label_option == LabelOption::AddLabel
                    {
                        self.add_label(app);
                    } else if self.save_project(&app.db)? {
                        return Ok(true);
                    }
                }
                _ => {}
            }
        }

        Ok(false)
    }

    fn render(&self, app: &App, frame: &mut Frame, area: Rect) {
        let colors = &app.config.colors.clone();

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

        self.inputs.title.render(
            frame,
            app,
            title_layout,
            self.focused_pane == FocusedPane::Title,
        );

        self.inputs.description.render(
            frame,
            app,
            description_layout,
            self.focused_pane == FocusedPane::Description,
        );

        self.render_labels(frame, app, fixed_width_label_layout);

        self.inputs.actions.render(
            frame,
            app,
            actions_layout,
            self.focused_pane == FocusedPane::Actions,
        );
    }
}

impl ProjectEditor {
    fn db_new_project(&self, db: &Database) -> Result<()> {
        let description = if self.inputs.description.input_string().chars().count() == 0 {
            None
        } else {
            Some(self.inputs.description.input_string())
        };

        let highest_position = db.get_highest_position("project")?;
        db.conn().execute(
            "INSERT INTO project (title, description, position, created_at, updated_at) VALUES \
             (?1, ?2, ?3, ?4, ?5)",
            (
                self.inputs.title.input_string(),
                description,
                highest_position + 1,
                DateTime::now(),
                DateTime::now(),
            ),
        )?;

        let new_project_id = db.last_row_id("project")?;

        self.db_new_labels(db, new_project_id)?;

        Ok(())
    }

    fn db_new_labels(&self, db: &Database, project_id: i32) -> Result<()> {
        for (i, label) in self.inputs.labels.iter().enumerate() {
            let query = "INSERT INTO project_label (project_id, title, color, position, \
                         created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)";
            db.conn().execute(
                query,
                (
                    project_id,
                    label.1.input_string(),
                    label.2.input_string(),
                    i,
                    DateTime::now(),
                    DateTime::now(),
                ),
            )?;
        }

        Ok(())
    }

    fn db_edit_project(&self, db: &Database) -> Result<()> {
        if let Some(data) = &self.data {
            let conn = db.conn();
            let query =
                "UPDATE project SET title = ?1, description = ?2, updated_at = ?3 WHERE id = ?4";
            let mut stmt = conn.prepare(query)?;
            stmt.execute((
                self.inputs.title.input_string(),
                self.inputs.description.input_string(),
                DateTime::now(),
                data.id,
            ))?;
            self.db_edit_labels(db, data.id)?;
        } else {
            panic!("project data was not set")
        }

        Ok(())
    }

    fn db_edit_labels(&self, db: &Database, project_id: i32) -> Result<()> {
        let conn = db.conn();
        for (i, label) in self.inputs.labels.iter().enumerate() {
            if let Some(label_id) = label.0 {
                let query = "UPDATE project_label SET title = ?1, color = ?2, updated_at = ?3 \
                             WHERE project_id = ?4 and id = ?5";
                let mut stmt = conn.prepare(query)?;
                stmt.execute((
                    label.1.input_string(),
                    label.2.input_string(),
                    DateTime::now(),
                    project_id,
                    label_id,
                ))?;
            } else {
                let query = "INSERT INTO project_label (project_id, title, color, position, \
                             created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)";
                conn.execute(
                    query,
                    (
                        project_id,
                        label.1.input_string(),
                        label.2.input_string(),
                        i,
                        DateTime::now(),
                        DateTime::now(),
                    ),
                )?;
            }
        }

        Ok(())
    }
}

impl ProjectEditor {
    fn focus_next(&mut self) {
        match self.focused_pane {
            FocusedPane::Title => self.focused_pane = FocusedPane::Description,
            FocusedPane::Description => {
                if self.inputs.labels.is_empty() {
                    self.focused_label_option = LabelOption::AddLabel;
                }
                self.focused_pane = FocusedPane::Labels;
            }
            FocusedPane::Labels => self.focused_pane = FocusedPane::Actions,
            FocusedPane::Actions => self.focused_pane = FocusedPane::Title,
        }
    }

    fn focus_prev(&mut self) {
        match self.focused_pane {
            FocusedPane::Title => self.focused_pane = FocusedPane::Actions,
            FocusedPane::Description => self.focused_pane = FocusedPane::Title,
            FocusedPane::Labels => self.focused_pane = FocusedPane::Description,
            FocusedPane::Actions => {
                if self.inputs.labels.is_empty() {
                    self.focused_label_option = LabelOption::AddLabel;
                }
                self.focused_pane = FocusedPane::Labels;
            }
        }
    }

    fn toggle_label_col(&mut self) {
        if self.focused_pane == FocusedPane::Labels
            && self.focused_label_option == LabelOption::Labels
        {
            self.label_col = match self.label_col {
                LabelCol::Title => LabelCol::Color,
                LabelCol::Color => LabelCol::Title,
            }
        }
    }

    fn next_label(&mut self) {
        if self.focused_label_option == LabelOption::Labels {
            if self.inputs.labels.len().saturating_sub(1) == self.selected_label {
                self.focused_label_option = LabelOption::AddLabel;
            } else {
                self.selected_label = self.selected_label.saturating_add(1);
            }
        } else {
            self.focused_label_option = LabelOption::Labels;
            self.selected_label = 0;
        }
    }

    fn prev_label(&mut self) {
        if self.focused_label_option == LabelOption::Labels {
            if self.selected_label == 0 {
                self.focused_label_option = LabelOption::AddLabel;
            } else {
                self.selected_label -= 1;
            }
        } else {
            self.focused_label_option = LabelOption::Labels;
            self.selected_label = self.inputs.labels.len() - 1;
        }
    }

    fn add_label(&mut self, app: &mut App) {
        let title_input = TextInput::new("Title")
            .title_as_placeholder()
            .inline()
            .required()
            .max(LABEL_TITLE_MAX_LENTH);
        let mut color_input = TextInput::new("Color")
            .title_as_placeholder()
            .inline()
            .required_len(LABEL_COLOR_REQUIRED_LENTH);
        color_input.input(app.config.colors.fg.to_string());
        self.inputs.labels.push((None, title_input, color_input));
        self.selected_label = self.inputs.labels.len().saturating_sub(1);
        self.focused_label_option = LabelOption::Labels;
        self.label_col = LabelCol::Title;
    }

    fn save_project(&mut self, db: &Database) -> Result<bool> {
        if self.focused_pane == FocusedPane::Actions {
            if self.inputs.actions.is_focused(Action::Save) {
                if self.new {
                    self.db_new_project(db)?;
                } else {
                    self.db_edit_project(db)?;
                }
                self.reset()
            } else if self.inputs.actions.is_focused(Action::Cancel) {
                self.reset()
            }
            return Ok(true);
        }
        Ok(false)
    }
}

impl ProjectEditor {
    #[allow(clippy::type_complexity)]
    fn render_labels(&self, frame: &mut Frame, app: &App, area: Rect) {
        let colors = &app.config.colors;

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
            .border_style(
                Style::default().fg(if self.focused_pane == FocusedPane::Labels {
                    colors.primary
                } else {
                    colors.border
                }),
            );

        frame.render_widget(block, area);

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

            label_input.1.render(
                frame,
                app,
                title_layout,
                is_focused && self.label_col == LabelCol::Title,
            );

            label_input.2.render(
                frame,
                app,
                color_layout,
                is_focused && self.label_col == LabelCol::Color,
            );

            let label_preview_input = Paragraph::new(format!(" {} ", label_input.1.input_string()))
                .fg(Color::from_str(&label_input.2.input_string()).unwrap_or(colors.bg));

            frame.render_widget(label_preview_input, preview_layout)
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

        frame.render_widget(add_label, add_label_layout);
    }

    pub fn set_new(mut self) -> Self {
        self.new = true;
        self.inputs.actions.buttons[0].1 = String::from("Create New Project");
        self
    }

    pub fn set_project(&mut self, db: &Database, project_id: i32) -> Result<()> {
        let conn = db.conn();
        let project_query = "SELECT id, title, description FROM project WHERE id = ?1";
        let mut project_stmt = conn.prepare(project_query)?;
        let mut project = project_stmt.query_row([project_id], |r| {
            Ok(ProjectData {
                id: r.get(0)?,
                title: r.get(1)?,
                description: r.get(2)?,
                labels: vec![],
            })
        })?;

        project.labels = self.db_get_labels(db, project_id)?;

        self.data = Some(project);

        if let Some(data) = &self.data {
            self.inputs.title.input(data.title.clone());
            self.inputs
                .description
                .input(if let Some(description) = &data.description {
                    description.clone()
                } else {
                    String::from("")
                });
        }

        Ok(())
    }

    fn db_get_labels(&mut self, db: &Database, project_id: i32) -> Result<Vec<ProjectLabel>> {
        let conn = db.conn();
        let query = "SELECT project_id, id, title, color FROM project_label WHERE project_id = ?1 \
                     ORDER BY position";
        let mut stmt = conn.prepare(query)?;
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
            let label = l?;

            let mut title_input = TextInput::new("Title")
                .title_as_placeholder()
                .inline()
                .required()
                .max(LABEL_TITLE_MAX_LENTH);
            title_input.input(label.title.clone());
            let mut color_input = TextInput::new("Color")
                .title_as_placeholder()
                .inline()
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
        self.inputs.actions.reset();
        self.inputs.title.reset();
        self.inputs.description.reset();
    }
}
