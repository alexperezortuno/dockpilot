use crate::{
    app::{App, Overlay, commands},
    tui::Palette,
};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
};

pub fn draw(f: &mut Frame, app: &App, area: Rect, colors: Palette) {
    let Overlay::Palette { query, selected } = &app.overlay else {
        return;
    };
    let entries = commands::filtered(app, query, app.policy, app.task_status != "idle");
    let width = area.width.clamp(1, 76);
    let height = area.height.clamp(1, 16);
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 3;
    let rect = Rect::new(x, y, width, height);
    f.render_widget(Clear, rect);
    let parts = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(2)])
        .split(rect);
    f.render_widget(
        Paragraph::new(format!(":{}", query)).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Command Palette"),
        ),
        parts[0],
    );
    let items = entries.iter().enumerate().map(|(index, entry)| {
        let status = if entry.enabled {
            ""
        } else {
            entry.reason.as_deref().unwrap_or("disabled")
        };
        let text = format!(
            "{}  [{}]  {} {}",
            entry.label, entry.shortcut, entry.description, status
        );
        let mut item = ListItem::new(text);
        if index == *selected {
            item = item.style(
                Style::default()
                    .bg(colors.selection)
                    .add_modifier(Modifier::BOLD),
            );
        }
        item
    });
    f.render_widget(
        List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("{} matches", entries.len())),
        ),
        parts[1],
    );
}
