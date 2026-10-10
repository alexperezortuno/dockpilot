use crate::{
    app::{App, notifications::NotificationKind},
    tui::Palette,
};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
};

pub fn draw(f: &mut Frame, app: &App, area: Rect, colors: Palette) {
    let items: Vec<_> = app.notifications.items().rev().take(3).collect();
    if items.is_empty() {
        return;
    }
    let height = (items.len() as u16 * 2).min(area.height);
    let width = area.width.min(72);
    let rect = Rect::new(
        area.x + area.width.saturating_sub(width),
        area.y,
        width,
        height,
    );
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(std::iter::repeat_n(Constraint::Length(2), items.len()))
        .split(rect);
    for (item, row) in items.into_iter().zip(rows.iter()) {
        let color = match item.kind {
            NotificationKind::Success => Color::Green,
            NotificationKind::Info => colors.accent,
            NotificationKind::Warning => Color::Yellow,
            NotificationKind::Error => Color::Red,
        };
        f.render_widget(
            Paragraph::new(item.message.clone())
                .style(Style::default().fg(color))
                .block(Block::default().borders(Borders::ALL)),
            *row,
        );
    }
}
