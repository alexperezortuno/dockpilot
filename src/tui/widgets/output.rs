use crate::app::App;
use crate::app::logs::{LogLevel, classify};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem},
};

pub fn draw(f: &mut Frame, app: &mut App, area: Rect) {
    let visible_height = area.height.saturating_sub(2) as usize;
    let showing_logs = !app.log_lines.is_empty();
    let showing_events = !showing_logs && !app.event_lines.is_empty();
    let lines = if showing_logs {
        app.filtered_log_lines()
    } else if showing_events {
        app.event_lines.iter().cloned().collect()
    } else {
        app.output_lines.iter().cloned().collect()
    };
    let total = lines.len();
    let start = app.output_scroll.min(total.saturating_sub(visible_height));
    let items = lines
        .iter()
        .skip(start)
        .map(|line| {
            let style = match classify(line) {
                LogLevel::Debug => Style::default().fg(Color::DarkGray),
                LogLevel::Info => Style::default(),
                LogLevel::Warn => Style::default().fg(Color::Yellow),
                LogLevel::Error => Style::default().fg(Color::Red),
            };
            ListItem::new(line.clone()).style(style)
        })
        .collect::<Vec<_>>();
    let title = if showing_logs {
        format!(
            " Logs ({} lines{}{}{}) ",
            total,
            if app.logs_paused { ", paused" } else { "" },
            app.log_level_filter
                .map(|level| format!(", >= {:?}", level))
                .unwrap_or_default(),
            if app.log_timestamps {
                ", timestamps"
            } else {
                ""
            }
        )
    } else if showing_events {
        format!(" Events ({} lines, {} alerts) ", total, app.alerts.len())
    } else {
        format!(" Output ({} lines) ", total)
    };
    f.render_widget(
        List::new(items).block(Block::default().borders(Borders::ALL).title(title)),
        area,
    );
}
