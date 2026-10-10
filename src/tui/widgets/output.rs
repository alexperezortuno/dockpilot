use crate::app::App;
use ratatui::{
    Frame,
    layout::Rect,
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
        .map(|line| ListItem::new(line.clone()))
        .collect::<Vec<_>>();
    let title = if showing_logs {
        format!(
            " Logs ({} lines{}) ",
            total,
            if app.logs_paused { ", paused" } else { "" }
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
