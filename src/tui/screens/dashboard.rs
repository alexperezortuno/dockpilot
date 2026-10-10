use crate::app::App;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, Paragraph, Sparkline},
};

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let text = match &app.dashboard {
        Some(data) => {
            let selected = match &data.selected_stats {
                Some(stats) => format!(
                    "Selected {}: CPU {:.2}% | memory {}/{} MB",
                    stats.id,
                    stats.cpu_percent,
                    stats.memory_usage / 1_048_576,
                    stats.memory_limit / 1_048_576
                ),
                None => "Selected container stats: unavailable".to_string(),
            };
            let metrics = app
                .latest_metrics
                .map(|sample| {
                    format!(
                        "Metrics: CPU {} | memory {}% | net RX {} | net TX {} | block R/W {}/{}",
                        sample
                            .cpu_percent
                            .map_or_else(|| "N/A".to_string(), |value| format!("{value:.1}%")),
                        sample
                            .memory_percent
                            .map_or_else(|| "N/A".to_string(), |value| format!("{value:.1}%")),
                        crate::app::monitoring::format_rate(sample.network_rx_per_second),
                        crate::app::monitoring::format_rate(sample.network_tx_per_second),
                        crate::app::monitoring::format_rate(sample.block_read_per_second),
                        crate::app::monitoring::format_rate(sample.block_write_per_second),
                    )
                })
                .unwrap_or_else(|| "Metrics: N/A".to_string());
            format!(
                "Docker Engine {}\nContainers: {} total | {} running | {} paused | {} stopped\nHealth: {} running, {} paused, {} stopped\n{}\n{}\nPress d to refresh dashboard",
                data.engine_version,
                data.containers_total,
                data.containers_running,
                data.containers_paused,
                data.containers_stopped,
                data.containers_running,
                data.containers_paused,
                data.containers_stopped,
                selected,
                metrics
            )
        }
        None => "Dashboard data unavailable\nPress d to refresh dashboard".to_string(),
    };
    let parts = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(3)])
        .split(area);
    f.render_widget(
        Paragraph::new(text).block(Block::default().borders(Borders::ALL).title("Dashboard")),
        parts[0],
    );
    let task_text = app
        .task_history
        .recent_summary(std::time::Instant::now())
        .join("\n");
    if !task_text.is_empty() {
        let task_area = Rect::new(
            area.x.saturating_add(area.width.saturating_sub(32)),
            area.y,
            32.min(area.width),
            area.height.min(5),
        );
        f.render_widget(
            Paragraph::new(task_text).block(Block::default().borders(Borders::ALL).title("Tasks")),
            task_area,
        );
    }
    let cpu: Vec<u64> = app
        .metrics_history
        .samples()
        .filter_map(|sample| sample.cpu_percent)
        .map(|value| value.round() as u64)
        .collect();
    f.render_widget(
        Sparkline::default()
            .block(Block::default().borders(Borders::ALL).title("CPU history"))
            .data(cpu),
        parts[1],
    );
}
