use crate::app::App;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
};

pub fn draw(f: &mut Frame, app: &mut App, area: Rect) {
    app.unified_logs_refresh_view();
    let viewport = area.height.saturating_sub(2) as usize;
    let sequences = app.unified_log_view.cached_sequences.clone();
    let selected = app.unified_log_view.selected_sequence;
    let selected_index = selected
        .and_then(|sequence| sequences.iter().position(|item| *item == sequence))
        .unwrap_or_else(|| sequences.len().saturating_sub(1));
    let start = selected_index
        .saturating_sub(viewport.saturating_sub(1))
        .min(sequences.len().saturating_sub(viewport));
    app.unified_log_view.scroll = start;
    let query = app.unified_log_view.search_query.to_ascii_lowercase();
    let items = sequences
        .iter()
        .skip(start)
        .take(viewport)
        .filter_map(|sequence| app.unified_log_buffer.get(*sequence))
        .map(|event| {
            let prefix = format!(
                "{} {} [{:?}] ",
                event.timestamp.as_deref().unwrap_or("-"),
                event.container_name,
                event.stream
            );
            let mut spans = vec![Span::raw(prefix)];
            if query.is_empty() {
                spans.push(Span::raw(event.message.clone()));
            } else {
                let lower = event.message.to_ascii_lowercase();
                let mut offset = 0;
                while let Some(found) = lower[offset..].find(&query) {
                    let start_match = offset + found;
                    spans.push(Span::raw(event.message[offset..start_match].to_string()));
                    let end_match = start_match + query.len();
                    spans.push(Span::styled(
                        event.message[start_match..end_match].to_string(),
                        Style::default().fg(Color::Yellow),
                    ));
                    offset = end_match;
                }
                spans.push(Span::raw(event.message[offset..].to_string()));
            }
            let style = if Some(event.sequence) == selected {
                Style::default().bg(Color::Blue)
            } else {
                Style::default()
            };
            ListItem::new(Line::from(spans)).style(style)
        });
    let filter = &app.unified_log_view.filter;
    let active = [
        filter
            .container
            .as_deref()
            .map(|value| format!("container={value}")),
        filter
            .compose_project
            .as_deref()
            .map(|value| format!("project={value}")),
        filter
            .compose_service
            .as_deref()
            .map(|value| format!("service={value}")),
        filter
            .minimum_severity
            .map(|value| format!("severity>={value:?}")),
        filter
            .since
            .as_deref()
            .map(|value| format!("since={value}")),
        filter
            .until
            .as_deref()
            .map(|value| format!("until={value}")),
        filter.text.as_deref().map(|value| format!("text={value}")),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" ");
    let mode = if app.unified_log_view.paused {
        "paused"
    } else {
        "live"
    };
    let follow = if app.unified_log_view.follow {
        "follow"
    } else {
        "history"
    };
    let position = if sequences.is_empty() {
        0
    } else {
        selected_index + 1
    };
    let dropped = app.unified_log_buffer.dropped();
    f.render_widget(
        List::new(items).block(Block::default().borders(Borders::ALL).title(format!(
            " Unified Logs {mode}/{follow} {position}/{} dropped={dropped} {} ",
            sequences.len(),
            if active.is_empty() { "" } else { &active }
        ))),
        area,
    );
}
