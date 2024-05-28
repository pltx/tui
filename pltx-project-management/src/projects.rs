use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use pltx_app::{App, Screen};
use ratatui::{layout::Rect, Frame};

use super::{
    list_projects::ListProjects, open_project::OpenProject, project_editor::ProjectEditor,
};

#[derive(PartialEq)]
enum Page {
    ListProjects,
    NewProject,
    EditProject,
    OpenProject,
}

struct Pages {
    list_projects: ListProjects,
    new_project: ProjectEditor,
    edit_project: ProjectEditor,
    open_project: OpenProject,
}

pub struct Projects {
    page: Page,
    pages: Pages,
}

impl Screen<Result<()>> for Projects {
    fn init(app: &App) -> Result<Projects> {
        Ok(Projects {
            page: Page::ListProjects,
            pages: Pages {
                list_projects: ListProjects::init(app)?,
                new_project: ProjectEditor::init(app)?.set_new(),
                edit_project: ProjectEditor::init(app)?,
                open_project: OpenProject::init(app)?,
            },
        })
    }

    fn key_event_handler(&mut self, app: &mut App, key_event: KeyEvent) -> Result<()> {
        if app.is_normal_mode() && self.page == Page::ListProjects {
            match key_event.code {
                KeyCode::Char('n') => self.page = Page::NewProject,
                KeyCode::Char('e') => {
                    if let Some(selected_id) = self.pages.list_projects.selected_id {
                        self.pages
                            .edit_project
                            .set_project(&app.db, selected_id)
                            .unwrap_or_else(|e| panic!("{e}"));
                        self.page = Page::EditProject;
                    }
                }
                KeyCode::Enter | KeyCode::Char('l') => {
                    if let Some(selected_id) = self.pages.list_projects.selected_id {
                        self.pages.open_project.reset(app);
                        self.pages.open_project.set_project_id(selected_id);
                        self.pages.open_project.db_get_project(app)?;
                        self.page = Page::OpenProject;
                    }
                }
                _ => {}
            }
        }

        let result: bool = match self.page {
            Page::ListProjects => self.pages.list_projects.key_event_handler(app, key_event)?,
            Page::NewProject => self.pages.new_project.key_event_handler(app, key_event)?,
            Page::EditProject => self.pages.edit_project.key_event_handler(app, key_event)?,
            Page::OpenProject => self.pages.open_project.key_event_handler(app, key_event)?,
        };

        if result {
            self.page = Page::ListProjects;
            self.pages.list_projects.db_get_projects(app)?;
        }

        Ok(())
    }

    fn render(&self, app: &App, frame: &mut Frame, area: Rect) {
        match self.page {
            Page::ListProjects => self.pages.list_projects.render(app, frame, area),
            Page::NewProject => self.pages.new_project.render(app, frame, area),
            Page::EditProject => self.pages.edit_project.render(app, frame, area),
            Page::OpenProject => self.pages.open_project.render(app, frame, area),
        }
    }
}
