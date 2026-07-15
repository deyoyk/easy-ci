pub mod app;
pub mod ui;

use crate::app::App;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use eci_core::state::State;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;

pub fn run_dashboard(state: &State) -> eci_core::error::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(state)?;

    loop {
        terminal.draw(|f| ui::draw(f, &app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    // Quit: q or Esc
                    KeyCode::Char('q') | KeyCode::Esc => {
                        app.should_quit = true;
                    }
                    // Tab: cycle through tabs
                    KeyCode::Tab => {
                        app.next_tab();
                    }
                    // Shift+Tab: cycle backwards through tabs
                    KeyCode::BackTab => {
                        app.previous_tab();
                    }
                    // Left/Right: also cycle tabs
                    KeyCode::Left => {
                        app.previous_tab();
                    }
                    KeyCode::Right => {
                        app.next_tab();
                    }
                    // Up/Down or j/k: navigate within current tab
                    KeyCode::Up | KeyCode::Char('k') => {
                        match app.active_tab {
                            crate::app::ActiveTab::Projects => app.previous_project(),
                            crate::app::ActiveTab::Apps => app.previous_app(),
                            _ => {}
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        match app.active_tab {
                            crate::app::ActiveTab::Projects => app.next_project(),
                            crate::app::ActiveTab::Apps => app.next_app(),
                            _ => {}
                        }
                    }
                    // Enter: select/expand item
                    KeyCode::Enter => {
                        // Future: open logs for selected app, etc.
                    }
                    _ => {}
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}
