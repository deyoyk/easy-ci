use eci_core::state::State;

#[derive(PartialEq, Clone, Copy)]
pub enum ActiveTab {
    Projects,
    Apps,
    Logs,
}

impl ActiveTab {
    pub fn next(self) -> Self {
        match self {
            ActiveTab::Projects => ActiveTab::Apps,
            ActiveTab::Apps => ActiveTab::Logs,
            ActiveTab::Logs => ActiveTab::Projects,
        }
    }

    pub fn previous(self) -> Self {
        match self {
            ActiveTab::Projects => ActiveTab::Logs,
            ActiveTab::Apps => ActiveTab::Projects,
            ActiveTab::Logs => ActiveTab::Apps,
        }
    }

    pub fn index(self) -> usize {
        match self {
            ActiveTab::Projects => 0,
            ActiveTab::Apps => 1,
            ActiveTab::Logs => 2,
        }
    }
}

pub struct App {
    pub projects: Vec<eci_core::types::Project>,
    pub apps: Vec<eci_core::types::App>,
    pub selected_project: usize,
    pub selected_app: usize,
    pub active_tab: ActiveTab,
    pub logs: Vec<String>,
    pub should_quit: bool,
}

impl App {
    pub fn new(state: &State) -> eci_core::error::Result<Self> {
        let projects = state.list_projects()?;
        let apps = state.list_apps()?;
        Ok(Self {
            projects,
            apps,
            selected_project: 0,
            selected_app: 0,
            active_tab: ActiveTab::Projects,
            logs: Vec::new(),
            should_quit: false,
        })
    }

    pub fn next_tab(&mut self) {
        self.active_tab = self.active_tab.next();
    }

    pub fn previous_tab(&mut self) {
        self.active_tab = self.active_tab.previous();
    }

    pub fn next_project(&mut self) {
        if !self.projects.is_empty() {
            self.selected_project = (self.selected_project + 1) % self.projects.len();
        }
    }

    pub fn previous_project(&mut self) {
        if !self.projects.is_empty() {
            self.selected_project = if self.selected_project == 0 {
                self.projects.len() - 1
            } else {
                self.selected_project - 1
            };
        }
    }

    pub fn next_app(&mut self) {
        if !self.apps.is_empty() {
            self.selected_app = (self.selected_app + 1) % self.apps.len();
        }
    }

    pub fn previous_app(&mut self) {
        if !self.apps.is_empty() {
            self.selected_app = if self.selected_app == 0 {
                self.apps.len() - 1
            } else {
                self.selected_app - 1
            };
        }
    }
}
