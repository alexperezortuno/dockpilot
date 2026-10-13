use crate::app::{App, unified_logs::LogFilter};
use ratatui::{
    Frame,
    layout::Rect,
    widgets::{Block, Borders, List, ListItem},
};

pub fn draw(f: &mut Frame, app: &mut App, area: Rect) {
    let events = app.unified_log_buffer.visible(&LogFilter::default());
    let items = events.iter().map(|event| {
        ListItem::new(format!(
            "{} {} [{}] {}",
            event.timestamp.as_deref().unwrap_or("-"),
            event.container_name,
            format!("{:?}", event.stream).to_ascii_lowercase(),
            event.message
        ))
    });
    f.render_widget(
        List::new(items).block(Block::default().borders(Borders::ALL).title(format!(
            " Unified Logs ({} events, approximate cross-container order) ",
            events.len()
        ))),
        area,
    );
}
