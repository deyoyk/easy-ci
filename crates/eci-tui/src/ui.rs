use crate::app::{ActiveTab, App};
use eci_core::types::AppStatus;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Tabs, Wrap};
use ratatui::Frame;
use ratatui::style::Stylize;

const BRAND: Color = Color::Rgb(108, 92, 231);
const SUCCESS: Color = Color::Rgb(0, 184, 148);
const WARNING: Color = Color::Rgb(253, 203, 110);
const DIM: Color = Color::Rgb(99, 110, 114);
const FG: Color = Color::Rgb(223, 230, 233);
const BORDER: Color = Color::Rgb(108, 92, 231);
const BORDER_DIM: Color = Color::Rgb(99, 110, 114);
const SELECTED_BG: Color = Color::Rgb(108, 92, 231);

pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Main layout: header + content + footer
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(area);

    draw_header(frame, chunks[0], app);
    draw_content(frame, chunks[1], app);
    draw_footer(frame, chunks[2]);
}

fn draw_header(frame: &mut Frame, area: Rect, app: &App) {
    let titles = vec![
        Line::from(Span::styled(" Projects ", Style::default().fg(if app.active_tab == ActiveTab::Projects { BRAND } else { DIM }))),
        Line::from(Span::styled(" Apps ", Style::default().fg(if app.active_tab == ActiveTab::Apps { BRAND } else { DIM }))),
        Line::from(Span::styled(" Logs ", Style::default().fg(if app.active_tab == ActiveTab::Logs { BRAND } else { DIM }))),
    ];

    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(BORDER))
                .title(Span::styled(" ⚡ easy-ci ", Style::default().fg(BRAND).bold()))
        )
        .select(app.active_tab.index())
        .style(Style::default().fg(DIM))
        .highlight_style(Style::default().fg(BRAND).add_modifier(Modifier::BOLD));

    frame.render_widget(tabs, area);
}

fn draw_content(frame: &mut Frame, area: Rect, app: &App) {
    // Split into left sidebar and main content
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(22),
            Constraint::Min(0),
        ])
        .split(area);

    draw_sidebar(frame, chunks[0], app);
    draw_main(frame, chunks[1], app);
}

fn draw_sidebar(frame: &mut Frame, area: Rect, app: &App) {
    let items: Vec<ListItem> = app
        .projects
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let is_selected = i == app.selected_project && app.active_tab == ActiveTab::Projects;
            let style = if is_selected {
                Style::default()
                    .fg(Color::White)
                    .bg(SELECTED_BG)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(FG)
            };
            let prefix = if is_selected { "▸ " } else { "  " };
            ListItem::new(Line::from(Span::styled(
                format!("{}{}", prefix, p.name),
                style,
            )))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(BORDER_DIM))
                .title(Span::styled(
                    format!(" Projects ({}) ", app.projects.len()),
                    Style::default().fg(DIM),
                ))
        );
    frame.render_widget(list, area);
}

fn draw_main(frame: &mut Frame, area: Rect, app: &App) {
    match app.active_tab {
        ActiveTab::Projects => draw_project_detail(frame, area, app),
        ActiveTab::Apps => draw_apps(frame, area, app),
        ActiveTab::Logs => draw_logs(frame, area, app),
    }
}

fn draw_project_detail(frame: &mut Frame, area: Rect, app: &App) {
    if let Some(project) = app.projects.get(app.selected_project) {
        let text = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("  Name: ", Style::default().fg(DIM)),
                Span::styled(&project.name, Style::default().fg(FG).bold()),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Description: ", Style::default().fg(DIM)),
                Span::styled(
                    project.description.as_deref().unwrap_or("—"),
                    Style::default().fg(FG),
                ),
            ]),
            Line::from(""),
        ];

        let paragraph = Paragraph::new(text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(BORDER_DIM))
                    .title(Span::styled(
                        format!(" {} ", project.name),
                        Style::default().fg(BRAND).bold(),
                    ))
            );
        frame.render_widget(paragraph, area);
    }
}

fn draw_apps(frame: &mut Frame, area: Rect, app: &App) {
    if app.apps.is_empty() {
        let text = vec![
            Line::from(""),
            Line::from(Span::styled("  No apps deployed yet", Style::default().fg(DIM))),
            Line::from(Span::styled("  Run: eci deploy", Style::default().fg(DIM))),
            Line::from(""),
        ];
        let paragraph = Paragraph::new(text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(BORDER_DIM))
                    .title(Span::styled(" Apps ", Style::default().fg(DIM)))
            );
        frame.render_widget(paragraph, area);
        return;
    }

    let header = Line::from(vec![
        Span::styled("  ", Style::default()),
        Span::styled(format!("{:<20}", "NAME"), Style::default().fg(DIM).add_modifier(Modifier::BOLD)),
        Span::styled("  ", Style::default()),
        Span::styled(format!("{:<10}", "STATUS"), Style::default().fg(DIM).add_modifier(Modifier::BOLD)),
        Span::styled("  ", Style::default()),
        Span::styled("IMAGE", Style::default().fg(DIM).add_modifier(Modifier::BOLD)),
    ]);

    let rows: Vec<Line> = app
        .apps
        .iter()
        .enumerate()
        .map(|(i, a)| {
            let (icon, color) = match a.status {
                AppStatus::Running => ("●", SUCCESS),
                AppStatus::Stopped => ("○", DIM),
                AppStatus::Unhealthy => ("◐", WARNING),
                AppStatus::Deploying => ("◑", BRAND),
            };

            let is_selected = i == app.selected_app;

            Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(
                    format!("{:<20}", a.name),
                    if is_selected { Style::default().fg(Color::White).bg(SELECTED_BG).add_modifier(Modifier::BOLD) } else { Style::default().fg(FG) },
                ),
                Span::styled("  ", Style::default()),
                Span::styled(
                    format!("{} {:<8}", icon, format!("{:?}", a.status)),
                    Style::default().fg(color),
                ),
                Span::styled("  ", Style::default()),
                Span::styled(
                    format!("{:<20}", a.image_tag),
                    Style::default().fg(DIM),
                ),
            ])
        })
        .collect();

    let mut lines = vec![header, Line::from("")];
    lines.extend(rows);

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(BORDER_DIM))
                .title(Span::styled(
                    format!(" Apps ({}) ", app.apps.len()),
                    Style::default().fg(DIM),
                ))
        );
    frame.render_widget(paragraph, area);
}

fn draw_logs(frame: &mut Frame, area: Rect, app: &App) {
    let text: Vec<Line> = if app.logs.is_empty() {
        vec![
            Line::from(""),
            Line::from(Span::styled("  Select an app to view logs", Style::default().fg(DIM))),
            Line::from(""),
        ]
    } else {
        app.logs
            .iter()
            .map(|l| Line::from(Span::styled(
                format!("  {}", l),
                Style::default().fg(FG),
            )))
            .collect()
    };

    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(BORDER_DIM))
                .title(Span::styled(" Logs ", Style::default().fg(DIM)))
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

fn draw_footer(frame: &mut Frame, area: Rect) {
    let footer = Line::from(vec![
        Span::styled("  Tab ", Style::default().fg(BRAND).bold()),
        Span::styled("switch  ", Style::default().fg(DIM)),
        Span::styled("↑↓ ", Style::default().fg(BRAND).bold()),
        Span::styled("navigate  ", Style::default().fg(DIM)),
        Span::styled("Esc ", Style::default().fg(BRAND).bold()),
        Span::styled("quit", Style::default().fg(DIM)),
    ]);

    let paragraph = Paragraph::new(footer)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(BORDER_DIM))
        );
    frame.render_widget(paragraph, area);
}
