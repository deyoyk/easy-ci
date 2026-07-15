use crate::app::{ActiveTab, App};
use eci_core::types::AppStatus;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Tabs};
use ratatui::Frame;

pub fn draw(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(frame.area());

    draw_header(frame, chunks[0], app);
    draw_content(frame, chunks[1], app);
    draw_footer(frame, chunks[2]);
}

fn draw_header(frame: &mut Frame, area: Rect, app: &App) {
    let titles = vec!["Projects", "Apps", "Logs"];
    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL).title("easy-ci"))
        .select(match app.active_tab {
            ActiveTab::Projects => 0,
            ActiveTab::Apps => 1,
            ActiveTab::Logs => 2,
        })
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
    frame.render_widget(tabs, area);
}

fn draw_content(frame: &mut Frame, area: Rect, app: &App) {
    match app.active_tab {
        ActiveTab::Projects => draw_projects(frame, area, app),
        ActiveTab::Apps => draw_apps(frame, area, app),
        ActiveTab::Logs => draw_logs(frame, area, app),
    }
}

fn draw_projects(frame: &mut Frame, area: Rect, app: &App) {
    let items: Vec<ListItem> = app
        .projects
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let style = if i == app.selected_project {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(Line::from(Span::styled(&p.name, style)))
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Projects"));
    frame.render_widget(list, area);
}

fn draw_apps(frame: &mut Frame, area: Rect, app: &App) {
    let header = Line::from(vec![Span::styled(
        format!("{:<20} {:<12} {:<20}", "NAME", "STATUS", "IMAGE"),
        Style::default().fg(Color::Yellow),
    )]);

    let rows: Vec<Line> = app
        .apps
        .iter()
        .enumerate()
        .map(|(i, a)| {
            let status_icon = match a.status {
                AppStatus::Running => "●",
                AppStatus::Stopped => "○",
                AppStatus::Unhealthy => "◐",
                AppStatus::Deploying => "◑",
            };
            let style = if i == app.selected_app {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            Line::from(Span::styled(
                format!(
                    "{:<20} {} {:<10} {:<20}",
                    a.name,
                    status_icon,
                    format!("{:?}", a.status),
                    a.image_tag
                ),
                style,
            ))
        })
        .collect();

    let mut lines = vec![header];
    lines.extend(rows);

    let paragraph = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title("Apps"));
    frame.render_widget(paragraph, area);
}

fn draw_logs(frame: &mut Frame, area: Rect, app: &App) {
    let text: Vec<Line> = app
        .logs
        .iter()
        .map(|l| Line::from(Span::raw(l)))
        .collect();

    let paragraph = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL).title("Logs"))
        .scroll((0, 0));
    frame.render_widget(paragraph, area);
}

fn draw_footer(frame: &mut Frame, area: Rect) {
    let footer = Line::from(vec![
        Span::styled(" F1:help ", Style::default().fg(Color::DarkGray)),
        Span::styled(" F2:projects ", Style::default().fg(Color::DarkGray)),
        Span::styled(" F3:apps ", Style::default().fg(Color::DarkGray)),
        Span::styled(" F4:logs ", Style::default().fg(Color::DarkGray)),
        Span::styled(" q:quit ", Style::default().fg(Color::DarkGray)),
    ]);
    let paragraph = Paragraph::new(footer)
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(paragraph, area);
}
