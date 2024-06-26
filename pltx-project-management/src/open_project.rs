use std::{collections::HashSet, str::FromStr, time::Instant};

use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use pltx_app::{state::AppPopup, App, DefaultWidget, KeyEventHandler, Popup, Screen};
use pltx_database::Database;
use pltx_utils::{DateTime, WidgetMargin};
use pltx_widgets::{Card, CardBorderType, Scrollable};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Padding, Paragraph, Widget},
    Frame,
};
use tracing::{info, info_span};

use crate::popups::{card_editor::CardEditor, card_viewer::CardViewer, list_editor::ListEditor};

#[derive(Clone)]
pub struct ProjectLabel {
    pub id: i32,
    pub title: String,
    pub color: String,
}

#[derive(Clone)]
struct ProjectCardLabel {
    card_id: i32,
    label_id: i32,
}

#[derive(Clone)]
struct ProjectCardSubtask {
    card_id: i32,
    completed: bool,
}

#[derive(Clone)]
struct OpenProjectCard {
    id: i32,
    list_id: i32,
    title: String,
    description: Option<String>,
    important: bool,
    start_date: Option<DateTime>,
    due_date: Option<DateTime>,
    completed: bool,
    position: i32,
    labels: HashSet<i32>,
    subtasks: Vec<ProjectCardSubtask>,
}

impl OpenProjectCard {
    fn in_progress(&self) -> bool {
        self.start_date.as_ref().is_some_and(|d| d.is_past()) && !self.overdue()
    }

    fn due_soon(&self, days: i32) -> bool {
        self.due_date.as_ref().is_some_and(|d| d.is_past_days(days)) && !self.overdue()
    }

    fn overdue(&self) -> bool {
        self.due_date.as_ref().is_some_and(|d| d.is_past())
    }
}

#[derive(Clone)]
struct ProjectList {
    id: i32,
    title: String,
    cards: Vec<OpenProjectCard>,
}

#[derive(Default, Clone)]
struct ProjectData {
    title: String,
    labels: Vec<ProjectLabel>,
    lists: Vec<ProjectList>,
}

#[derive(PartialEq)]
enum OpenProjectPopup {
    NewList,
    EditList,
    ViewCard,
    NewCard,
    EditCard,
    None,
}

struct Popups {
    new_list: ListEditor,
    edit_list: ListEditor,
    view_card: CardViewer,
    new_card: CardEditor,
    edit_card: CardEditor,
}

#[derive(PartialEq)]
enum DeleteSelection {
    List,
    Card,
    None,
}

#[derive(PartialEq)]
enum Focus {
    List,
    Card,
}

pub struct OpenProject {
    project_id: Option<i32>,
    selected_list_index: usize,
    data: ProjectData,
    popup: OpenProjectPopup,
    popups: Popups,
    delete_selection: DeleteSelection,
    list_selections: Vec<Scrollable>,
    focus: Focus,
}

impl Screen<Result<bool>> for OpenProject {
    fn init(_: &App) -> Result<OpenProject> {
        Ok(OpenProject {
            project_id: None,
            selected_list_index: 0,
            data: ProjectData::default(),
            popup: OpenProjectPopup::None,
            popups: Popups {
                new_list: ListEditor::init(),
                edit_list: ListEditor::init(),
                view_card: CardViewer::init(),
                new_card: CardEditor::init(),
                edit_card: CardEditor::init(),
            },
            delete_selection: DeleteSelection::None,
            list_selections: vec![],
            focus: Focus::Card,
        })
    }

    fn key_event_handler(&mut self, app: &mut App, key_event: KeyEvent) -> Result<bool> {
        if app.view.is_popup() {
            match self.popup {
                OpenProjectPopup::NewList => {
                    if self.popups.new_list.key_event_handler(app, key_event)? {
                        self.db_get_project(app)?;
                        let last_list_index = self.data.lists.len() - 1;
                        self.selected_list_index = last_list_index;
                    }
                }
                OpenProjectPopup::EditList => {
                    if self.popups.edit_list.key_event_handler(app, key_event)? {
                        self.db_get_project(app)?
                    }
                }
                OpenProjectPopup::ViewCard => {
                    if self.popups.view_card.key_event_handler(app, key_event)? {
                        self.db_get_project(app)?;
                    }
                }
                OpenProjectPopup::NewCard => {
                    if self.popups.new_card.key_event_handler(app, key_event)? {
                        self.db_get_project(app)?;
                        let last_card_index =
                            self.data.lists[self.selected_list_index].cards.len() - 1;
                        self.list_selections[self.selected_list_index].focused = last_card_index;
                    }
                }
                OpenProjectPopup::EditCard => {
                    if self.popups.edit_card.key_event_handler(app, key_event)? {
                        self.db_get_project(app)?
                    }
                }
                OpenProjectPopup::None => {}
            };
        }

        if app.view.is_default() && app.mode.is_normal() {
            match key_event.code {
                KeyCode::Char('[') => return Ok(true),
                KeyCode::Char('h') => {
                    if self.selected_list_index != 0 {
                        self.selected_list_index -= 1;
                    }
                }
                KeyCode::Char('l') => {
                    if self.selected_list_index != self.data.lists.len().saturating_sub(1) {
                        self.selected_list_index += 1;
                    }
                }
                _ => {}
            }

            if self.focus == Focus::List {
                match key_event.code {
                    KeyCode::Char('H') => self.decrement_list_position(app)?,
                    KeyCode::Char('L') => self.increment_list_position(app)?,
                    KeyCode::Char('j') => {
                        self.focus = Focus::Card;
                    }
                    KeyCode::Char('d') => {
                        if self.project_id.is_some() && !self.data.lists.is_empty() {
                            self.delete_selection = DeleteSelection::List;
                            app.mode.delete();
                        }
                    }
                    KeyCode::Char('e') => {
                        if !self.data.lists.is_empty() {
                            let list_id = self.data.lists[self.selected_list_index].id;
                            self.popup = OpenProjectPopup::EditList;
                            self.popups.edit_list.set(&app.db, list_id)?;
                            app.view.popup();
                            app.mode.insert();
                        }
                    }
                    KeyCode::Char('n') => {
                        self.popup = OpenProjectPopup::NewList;
                        app.view.popup();
                        app.mode.insert();
                    }
                    _ => {}
                }
            } else if self.focus == Focus::Card
                && self.data.lists.is_empty()
                && key_event.code == KeyCode::Char('n')
            {
                self.popup = OpenProjectPopup::NewList;
                app.view.popup();
                app.mode.insert();
            } else if self.focus == Focus::Card && !self.data.lists.is_empty() {
                if self.list_selections[self.selected_list_index].focused == 0
                    && key_event.code == KeyCode::Char('k')
                {
                    self.focus = Focus::List;
                } else {
                    self.list_selections[self.selected_list_index]
                        .key_event_handler(app, key_event);
                }

                match key_event.code {
                    KeyCode::Char('J') => self.increment_card_position(app)?,
                    KeyCode::Char('K') => self.decrement_card_position(app)?,
                    KeyCode::Char('H') => self.move_card_left(app)?,
                    KeyCode::Char('L') => self.move_card_right(app)?,
                    KeyCode::Enter => {
                        if !self.data.lists.is_empty()
                            && !self.data.lists[self.selected_list_index].cards.is_empty()
                        {
                            self.popup = OpenProjectPopup::ViewCard;
                            let card_index = self.list_selections[self.selected_list_index].focused;
                            let card_id =
                                self.data.lists[self.selected_list_index].cards[card_index].id;
                            self.popups.view_card.id(card_id);
                            self.popups.view_card.set_data(&app.db, card_id)?;
                            app.view.popup();
                        }
                    }
                    KeyCode::Char('n') => {
                        if let Some(project_id) = self.project_id {
                            if !self.data.lists.is_empty() {
                                let list_id = self.data.lists[self.selected_list_index].id;
                                self.popups.new_card.ids(project_id, list_id);
                                self.popup = OpenProjectPopup::NewCard;
                                app.view.popup();
                            }
                        }
                    }
                    KeyCode::Char('e') => {
                        if let Some(project_id) = self.project_id {
                            let selected_list_has_cards =
                                !self.data.lists[self.selected_list_index].cards.is_empty();
                            if !self.data.lists.is_empty() && selected_list_has_cards {
                                let list_id = self.data.lists[self.selected_list_index].id;
                                self.popups.edit_card.ids(project_id, list_id);
                                self.popup = OpenProjectPopup::EditCard;
                                let card_index =
                                    self.list_selections[self.selected_list_index].focused;
                                let card_id =
                                    self.data.lists[self.selected_list_index].cards[card_index].id;
                                self.popups.edit_card.set_data(&app.db, card_id)?;
                                app.view.popup();
                            }
                        }
                    }
                    KeyCode::Char('c') => self.db_toggle_card_completed(app)?,
                    KeyCode::Char('i') => self.db_toggle_card_important(app)?,
                    KeyCode::Char('d') => {
                        if !self.data.lists.is_empty()
                            && !self.data.lists[self.selected_list_index].cards.is_empty()
                        {
                            self.delete_selection = DeleteSelection::Card;
                            app.mode.delete();
                        }
                    }
                    _ => {}
                }
            }
        }

        if app.mode.is_delete() {
            match key_event.code {
                KeyCode::Char('y') => {
                    if self.delete_selection == DeleteSelection::List {
                        if !self.data.lists.is_empty() {
                            self.db_delete_list(&app.db)?;
                            self.db_get_project(app)?;
                            app.mode.normal();
                        }
                    } else if self.delete_selection == DeleteSelection::Card {
                        self.db_delete_card(&app.db)?;
                        self.db_get_project(app)?;
                        app.mode.normal();
                    }
                    self.delete_selection = DeleteSelection::None;
                }
                KeyCode::Char('n') => app.mode.normal(),

                _ => {}
            }
        }
        Ok(false)
    }

    fn render(&self, app: &App, frame: &mut Frame, area: Rect) {
        let colors = &app.config.colors.clone();

        let [title_area, list_areas] = Layout::default()
            .horizontal_margin(1)
            .constraints([Constraint::Length(3), Constraint::Fill(1)])
            .areas(area);

        let title = Paragraph::new(Line::from(vec![
            Span::from("Project: ").fg(colors.secondary_fg),
            Span::from(self.data.title.to_string()),
        ]))
        .block(
            Block::new()
                .padding(Padding::horizontal(1))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::new().fg(colors.border)),
        );

        frame.render_widget(title, title_area);

        if self.data.lists.is_empty() {
            let content = Paragraph::new(Text::from(vec![Line::from(vec![
                Span::from("You have no lists in your project. Press "),
                Span::styled("n", Style::new().bold().fg(colors.keybind_key)),
                Span::from(" to create a new list."),
            ])]))
            .block(Block::new().padding(Padding::horizontal(1)));

            frame.render_widget(content, list_areas)
        } else {
            let project_layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(
                    self.data
                        .lists
                        .iter()
                        .map(|_| Constraint::Fill(1))
                        .collect::<Vec<Constraint>>(),
                )
                .split(list_areas);

            for (list_index, list_layout) in project_layout.iter().enumerate() {
                let list_width = list_areas.width as usize - 2;
                let list = &self.data.lists[list_index];

                let list_card = Card::new(&format!(" {} ", list.title), *list_layout)
                    .focused_title(self.focus == Focus::List)
                    .border_type(CardBorderType::Rounded)
                    .margin(if list_index == 0 {
                        WidgetMargin::zero()
                    } else {
                        WidgetMargin::left(1)
                    });

                list_card.render(
                    frame,
                    app,
                    *list_layout,
                    self.selected_list_index == list_index,
                );

                if list.cards.is_empty() {
                    frame.render_widget(
                        Line::from(vec![
                            Span::from("There are no tasks in this list. Press "),
                            Span::from("n").bold().fg(colors.keybind_key),
                            Span::from(" to create a new task."),
                        ]),
                        list_card.child_layout(),
                    );
                } else {
                    let mut table = vec![];

                    for (card_index, card) in list.cards.iter().enumerate() {
                        let card = self.render_card(app, card, list_index, card_index, list_width);
                        table.push(card);
                    }

                    self.list_selections[list_index].render(frame, list_card.child_layout(), table);
                }
            }
        }

        if app.view.is_popup() && app.popup == AppPopup::None {
            match self.popup {
                OpenProjectPopup::NewList => self.popups.new_list.render(app, frame, list_areas),
                OpenProjectPopup::EditList => self.popups.edit_list.render(app, frame, list_areas),
                OpenProjectPopup::ViewCard => self.popups.view_card.render(app, frame, list_areas),
                OpenProjectPopup::NewCard => self.popups.new_card.render(app, frame, list_areas),
                OpenProjectPopup::EditCard => self.popups.edit_card.render(app, frame, list_areas),
                OpenProjectPopup::None => {}
            }
        }
    }
}

impl OpenProject {
    fn render_card(
        &self,
        app: &App,
        card: &OpenProjectCard,
        list_index: usize,
        card_index: usize,
        list_width: usize,
    ) -> impl Widget {
        let colors = &app.config.colors;

        let selected = self.selected_list_index == list_index
            && self.list_selections[list_index].focused == card_index;
        let unfocused_selected = self.selected_list_index != list_index
            && self.list_selections[list_index].focused == card_index;

        let config = &app.config.modules.project_management;
        let status_char = if card.completed {
            &config.completed_char
        } else if card.overdue() {
            &config.overdue_char
        } else if card.due_soon(app.config.modules.project_management.due_soon_days) {
            &config.due_soon_char
        } else if card.in_progress() {
            &config.in_progress_char
        } else if card.important {
            &config.important_char
        } else {
            &config.default_char
        };

        let line_style =
            if self.selected_list_index == list_index && selected && self.focus == Focus::Card {
                Style::new().bold().fg(colors.fg).bg(colors.input_focus_bg)
            } else if unfocused_selected {
                Style::new().bold().fg(colors.fg)
            } else {
                Style::new().fg(colors.secondary_fg)
            };

        let title = Line::from(vec![
            Span::from(format!(" [{}] ", status_char)).fg(
                if self.selected_list_index == list_index && selected {
                    colors.fg
                } else {
                    colors.secondary_fg
                },
            ),
            if card.completed {
                Span::from(card.title.to_string())
                    .fg(colors.secondary_fg)
                    .add_modifier(Modifier::CROSSED_OUT)
            } else {
                Span::from(card.title.to_string()).fg(colors.fg)
            },
            Span::from(" ".repeat(list_width.saturating_sub(card.title.chars().count() + 2))),
        ])
        .style(line_style);

        let mut details = vec![Span::from(" ".repeat(5)).fg(colors.tertiary_fg)];

        if card.description.is_some() {
            details.push(Span::from(" ≡").fg(colors.secondary_fg));
        }

        if !card.subtasks.is_empty() {
            details.push(
                Span::from(format!(
                    " {}",
                    card.subtasks.iter().filter(|st| st.completed).count(),
                ))
                .fg(colors.success),
            );
            details.push(Span::from("/").fg(colors.secondary_fg));
            details.push(Span::from(card.subtasks.len().to_string()));
            details.push(Span::from(" "));
        }

        for label in self.data.labels.iter() {
            if card.labels.contains(&label.id) {
                details.push(
                    Span::from(" ⬤")
                        .fg(Color::from_str(&label.color).expect("failed to parse label color")),
                );
            }
        }

        details.push(Span::from(" ".repeat(list_width.saturating_sub(
            card.labels.len() + if card.labels.is_empty() { 1 } else { 2 },
        ))));

        let details_line = Line::from(details).style(line_style);

        Paragraph::new(vec![title, details_line])
    }

    pub fn set_project_id(&mut self, project_id: i32) {
        self.project_id = Some(project_id);
        self.popups.new_list.project_id(project_id);
        self.popups.edit_list.project_id(project_id);
    }

    pub fn reset(&mut self, app: &mut App) {
        self.project_id = None;
        self.selected_list_index = 0;
        self.list_selections = vec![];
        self.data = ProjectData::default();
        self.popup = OpenProjectPopup::None;
        self.popups.new_list.reset(app);
        self.popups.edit_list.reset(app);
        self.popups.view_card.reset();
        self.popups.new_card.reset();
        self.popups.edit_card.reset();
        self.delete_selection = DeleteSelection::None;
    }
}

impl OpenProject {
    pub fn db_get_project(&mut self, app: &App) -> Result<()> {
        let start = Instant::now();
        let _span = info_span!("project management", screen = "open project").entered();

        let conn = app.db.conn();
        let query = "SELECT title, description, position, created_at, updated_at FROM project \
                     WHERE id = ?1 ORDER BY position";
        let mut stmt = conn.prepare(query)?;

        if let Some(project_id) = self.project_id {
            let mut project = stmt.query_row([project_id], |r| {
                Ok(ProjectData {
                    title: r.get(0)?,
                    labels: vec![],
                    lists: vec![],
                })
            })?;

            info!("get project query executed in {:?}", start.elapsed());

            project.labels = self.db_get_labels(app)?;
            project.lists = self.db_get_lists(&app.db, project_id)?;
            project = self.db_get_cards(&app.db, &mut project, project_id)?;
            project = self.db_get_card_labels(&app.db, &mut project, project_id)?;
            project = self.db_get_card_subtasks(&app.db, &mut project, project_id)?;

            if !project.lists.is_empty() {
                let list_id = project.lists[self.selected_list_index].id;

                self.popups.edit_list.set(&app.db, list_id)?;

                if let Some(project_id) = self.project_id {
                    self.popups.new_card.ids(project_id, list_id);
                    self.popups.edit_card.ids(project_id, list_id);
                }
            }

            self.data = project;
        }

        info!(
            "get project query durations totaled at {:?}",
            start.elapsed()
        );

        Ok(())
    }

    fn db_get_labels(&mut self, app: &App) -> Result<Vec<ProjectLabel>> {
        let start = Instant::now();
        let mut labels = vec![];

        let conn = app.db.conn();
        let project_label_query = "SELECT id, title, color, position, created_at, updated_at FROM \
                                   project_label WHERE project_id = ?1 ORDER BY position";
        let mut project_label_stmt = conn.prepare(project_label_query)?;

        let project_label_iter = project_label_stmt.query_map([&self.project_id], |r| {
            Ok(ProjectLabel {
                id: r.get(0)?,
                title: r.get(1)?,
                color: r.get(2)?,
            })
        })?;

        for label in project_label_iter {
            labels.push(label?);
        }

        self.popups.view_card.labels(labels.clone());
        self.popups
            .new_card
            .labels(&app.config.colors, labels.clone());
        self.popups
            .edit_card
            .labels(&app.config.colors, labels.clone());

        info!("get project labels query executed in {:?}", start.elapsed());

        Ok(labels)
    }

    fn db_get_lists(&mut self, db: &Database, project_id: i32) -> Result<Vec<ProjectList>> {
        let start = Instant::now();
        let mut lists = vec![];

        let conn = db.conn();
        let query =
            "SELECT id, title, position FROM project_list WHERE project_id = ?1 ORDER BY position";
        let mut stmt = conn.prepare(query)?;
        let project_list_iter = stmt.query_map([project_id], |r| {
            Ok(ProjectList {
                id: r.get(0)?,
                title: r.get(1)?,
                cards: vec![],
            })
        })?;
        for list in project_list_iter {
            lists.push(list?);
            self.list_selections
                .push(Scrollable::default().row_height(2));
        }

        info!("get project lists query executed in {:?}", start.elapsed());

        Ok(lists)
    }

    fn db_get_cards(
        &self,
        db: &Database,
        project: &mut ProjectData,
        project_id: i32,
    ) -> Result<ProjectData> {
        let start = Instant::now();
        let conn = db.conn();
        let project_card_query = "SELECT id, list_id, title, description, important, start_date, \
                                  due_date, completed, position FROM project_card WHERE \
                                  project_id = ?1 ORDER BY position";
        let mut project_card_stmt = conn.prepare(project_card_query)?;
        let project_card_iter = project_card_stmt.query_map([project_id], |r| {
            Ok(OpenProjectCard {
                id: r.get(0)?,
                list_id: r.get(1)?,
                title: r.get(2)?,
                description: r.get(3)?,
                important: r.get(4)?,
                start_date: DateTime::from_db_option(r.get(5)?),
                due_date: DateTime::from_db_option(r.get(6)?),
                completed: r.get(7)?,
                position: r.get(8)?,
                labels: HashSet::new(),
                subtasks: vec![],
            })
        })?;
        for card in project_card_iter {
            let c = card?;
            let index = project
                .lists
                .iter()
                .position(|l| l.id == c.list_id)
                .expect("failed to get project list index");
            project.lists[index].cards.push(c);
        }

        info!("get project cards query executed in {:?}", start.elapsed());

        Ok(project.clone())
    }

    fn db_get_card_labels(
        &self,
        db: &Database,
        project: &mut ProjectData,
        project_id: i32,
    ) -> Result<ProjectData> {
        let start = Instant::now();

        let conn = db.conn();
        let card_label_query = "SELECT card_id, label_id FROM card_label WHERE project_id = ?1";
        let mut card_label_stmt = conn.prepare(card_label_query)?;
        let card_label_iter = card_label_stmt.query_map([project_id], |r| {
            Ok(ProjectCardLabel {
                card_id: r.get(0)?,
                label_id: r.get(1)?,
            })
        })?;

        for card_label in card_label_iter {
            let label = card_label?;

            let list_index = project
                .lists
                .iter()
                .position(|l| l.cards.iter().any(|c| c.id == label.card_id))
                .expect("failed to get project list index");
            let card_index = project.lists[list_index]
                .cards
                .iter()
                .position(|c| c.id == label.card_id)
                .expect("failed to get project card index");
            project.lists[list_index].cards[card_index]
                .labels
                .insert(label.label_id);
        }

        info!("get card labels query executed in {:?}", start.elapsed());

        Ok(project.clone())
    }

    fn db_get_card_subtasks(
        &self,
        db: &Database,
        project: &mut ProjectData,
        project_id: i32,
    ) -> Result<ProjectData> {
        let start = Instant::now();

        let conn = db.conn();
        let card_subtask_query =
            "SELECT card_id, completed FROM card_subtask WHERE project_id = ?1";
        let mut card_subtask_stmt = conn.prepare(card_subtask_query)?;
        let card_subtask_iter = card_subtask_stmt.query_map([project_id], |r| {
            Ok(ProjectCardSubtask {
                card_id: r.get(0)?,
                completed: r.get(1)?,
            })
        })?;

        for card_subtask in card_subtask_iter {
            let subtask = card_subtask?;
            let list_index = project
                .lists
                .iter()
                .position(|l| l.cards.iter().any(|c| c.id == subtask.card_id))
                .expect("failed to get project list index");
            let card_index = project.lists[list_index]
                .cards
                .iter()
                .position(|c| c.id == subtask.card_id)
                .expect("failed to get project card index");
            project.lists[list_index].cards[card_index]
                .subtasks
                .push(subtask);
        }

        info!("get card subtasks query executed in {:?}", start.elapsed());

        Ok(project.clone())
    }

    fn db_delete_list(&mut self, db: &Database) -> Result<()> {
        let start = Instant::now();

        let list_id = self.data.lists[self.selected_list_index].id;
        let original_position = db.get_position("project_list", list_id)?;

        let query = "DELETE FROM project_list WHERE id = ?1";
        db.execute(query, [list_id])?;

        db.decrement_positions_after("project_list", original_position)?;

        if self.selected_list_index != 0 {
            self.selected_list_index -= 1;
        }

        info!(
            "delete project list query executed in {:?}",
            start.elapsed()
        );

        Ok(())
    }

    fn db_delete_card(&mut self, db: &Database) -> Result<()> {
        let start = Instant::now();

        let card_index = self.list_selections[self.selected_list_index].focused;
        let card = self.data.lists[self.selected_list_index].cards[card_index].clone();

        let original_position = db.get_position("project_card", card.id)?;

        let query = "DELETE FROM project_card WHERE id = ?1";
        db.execute(query, [card.id])?;

        db.decrement_positions_after("project_card", original_position)?;

        let list = &self.data.lists[self.selected_list_index];

        let selected_card_index = self.list_selections[self.selected_list_index].focused;
        if list.cards.len() == 1 {
            self.list_selections[self.selected_list_index].focused = 0;
        } else if selected_card_index == list.cards.len().saturating_sub(1) {
            self.list_selections[self.selected_list_index].focused -= 1;
        }

        info!(
            "delete project card query executed in {:?}",
            start.elapsed()
        );

        Ok(())
    }

    fn db_toggle_card_completed(&mut self, app: &App) -> Result<()> {
        let start = Instant::now();

        if let Some(card) = self.get_card() {
            let query = "UPDATE project_card SET completed = ?1, updated_at = ?2 WHERE id = ?3";
            let params = (!card.completed, DateTime::now(), card.id);
            app.db.execute(query, params)?;

            self.db_get_project(app)?;

            info!(
                "toggle project card completed query executed in {:?}",
                start.elapsed()
            );
        }

        Ok(())
    }

    fn db_toggle_card_important(&mut self, app: &App) -> Result<()> {
        let start = Instant::now();

        if let Some(card) = self.get_card() {
            let query = "UPDATE project_card SET important = ?1, updated_at = ?2 WHERE id = ?3";
            let params = (!card.important, DateTime::now(), card.id);
            app.db.execute(query, params)?;

            self.db_get_project(app)?;

            info!(
                "toggle project card important query executed in {:?}",
                start.elapsed()
            );
        }

        Ok(())
    }

    fn get_card(&self) -> Option<&OpenProjectCard> {
        self.list_selections
            .get(self.selected_list_index)
            .and_then(|card_indexes| {
                self.data.lists[self.selected_list_index]
                    .cards
                    .get(card_indexes.focused)
            })
    }

    fn increment_list_position(&mut self, app: &App) -> Result<()> {
        let _span = info_span!("project management", screen = "open project").entered();
        let start = Instant::now();
        if self.selected_list_index + 1 != self.data.lists.len() {
            let id = self.data.lists[self.selected_list_index].id;
            let next_id = self.data.lists[self.selected_list_index + 1].id;
            app.db.increment_position("project_list", id, next_id)?;
            self.selected_list_index += 1;
            info!(
                "increment list position query executed in {:?}",
                start.elapsed()
            );
            self.db_get_project(app)?;
        }
        Ok(())
    }

    fn decrement_list_position(&mut self, app: &App) -> Result<()> {
        let _span = info_span!("project management", screen = "open project").entered();
        let start = Instant::now();
        if self.selected_list_index != 0 {
            let id = self.data.lists[self.selected_list_index].id;
            let prev_id = self.data.lists[self.selected_list_index - 1].id;
            app.db.decrement_position("project_list", id, prev_id)?;
            self.selected_list_index -= 1;
            info!(
                "decrement list position query executed in {:?}",
                start.elapsed()
            );
            self.db_get_project(app)?;
        }
        Ok(())
    }

    fn increment_card_position(&mut self, app: &App) -> Result<()> {
        let _span = info_span!("project management", screen = "open project").entered();
        let start = Instant::now();
        if let Some(card_index) = self
            .list_selections
            .get(self.selected_list_index)
            .map(|l| l.focused)
        {
            if card_index + 1 != self.data.lists[self.selected_list_index].cards.len() {
                let id = self.data.lists[self.selected_list_index].cards[card_index].id;
                let next_id = self.data.lists[self.selected_list_index].cards[card_index + 1].id;
                app.db.increment_position("project_card", id, next_id)?;
                self.list_selections[self.selected_list_index].focused += 1;
                info!(
                    "increment card position query executed in {:?}",
                    start.elapsed()
                );
                self.db_get_project(app)?;
            }
        }
        Ok(())
    }

    fn decrement_card_position(&mut self, app: &App) -> Result<()> {
        let _span = info_span!("project management", screen = "open project").entered();
        let start = Instant::now();
        if let Some(card_index) = self
            .list_selections
            .get(self.selected_list_index)
            .map(|l| l.focused)
        {
            if card_index != 0 {
                let id = self.data.lists[self.selected_list_index].cards[card_index].id;
                let prev_id = self.data.lists[self.selected_list_index].cards[card_index - 1].id;
                app.db.decrement_position("project_card", id, prev_id)?;
                self.list_selections[self.selected_list_index].focused -= 1;
                info!(
                    "decrement card position query executed in {:?}",
                    start.elapsed()
                );
                self.db_get_project(app)?;
            }
        }
        Ok(())
    }

    fn move_card_left(&mut self, app: &App) -> Result<()> {
        let _span = info_span!("project management", screen = "open project").entered();
        let start = Instant::now();
        if let Some(card_index) = self
            .list_selections
            .get(self.selected_list_index)
            .map(|l| l.focused)
        {
            if self.selected_list_index != 0 {
                let original_list = &self.data.lists[self.selected_list_index];
                let left_list = &self.data.lists[self.selected_list_index - 1];
                let left_list_last_position =
                    left_list.cards.last().map(|l| l.position).unwrap_or(-1);

                app.db.execute(
                    "UPDATE project_card SET list_id = ?1, position = ?2 where list_id = ?3 and \
                     id = ?4",
                    [
                        left_list.id,
                        left_list_last_position + 1,
                        original_list.id,
                        original_list.cards[card_index].id,
                    ],
                )?;

                app.db.decrement_positions_after_where(
                    "project_card",
                    card_index as i32,
                    "list_id",
                    original_list.id,
                )?;

                self.list_selections[self.selected_list_index].focused = self.list_selections
                    [self.selected_list_index]
                    .focused
                    .saturating_sub(1);
                self.list_selections[self.selected_list_index - 1].focused =
                    left_list_last_position as usize + 1;
                self.selected_list_index -= 1;
                info!("move card left query executed in {:?}", start.elapsed());
                self.db_get_project(app)?;
            }
        }
        Ok(())
    }

    fn move_card_right(&mut self, app: &App) -> Result<()> {
        let _span = info_span!("project management", screen = "open project").entered();
        let start = Instant::now();
        if let Some(card_index) = self
            .list_selections
            .get(self.selected_list_index)
            .map(|l| l.focused)
        {
            if self.selected_list_index + 1 != self.data.lists.len() {
                let list = &self.data.lists[self.selected_list_index];
                let right_list = &self.data.lists[self.selected_list_index + 1];
                let right_list_last_position =
                    right_list.cards.last().map(|c| c.position).unwrap_or(-1);

                let query = "UPDATE project_card SET list_id = ?1, position = ?2 where list_id = \
                             ?3 and id = ?4";
                let params = [
                    right_list.id,
                    right_list_last_position + 1,
                    list.id,
                    list.cards[card_index].id,
                ];
                app.db.execute(query, params)?;

                app.db.decrement_positions_after_where(
                    "project_card",
                    card_index as i32,
                    "list_id",
                    list.id,
                )?;

                self.list_selections[self.selected_list_index].focused = self.list_selections
                    [self.selected_list_index]
                    .focused
                    .saturating_sub(1);
                self.list_selections[self.selected_list_index + 1].focused =
                    (right_list_last_position + 1) as usize;
                self.selected_list_index += 1;
                info!("move card right query executed in {:?}", start.elapsed());
                self.db_get_project(app)?;
            }
        }
        Ok(())
    }
}
