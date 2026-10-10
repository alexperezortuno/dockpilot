use crate::app::App;
use ratatui::{
    Frame,
    layout::Rect,
    widgets::{Block, Borders, Paragraph},
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
            format!(
                "Docker Engine {}\nContainers: {} total | {} running | {} paused | {} stopped\nHealth: {} running, {} paused, {} stopped\n{}\n\nPress d to refresh dashboard",
                data.engine_version,
                data.containers_total,
                data.containers_running,
                data.containers_paused,
                data.containers_stopped,
                data.containers_running,
                data.containers_paused,
                data.containers_stopped,
                selected
            )
        }
        None => "Dashboard data unavailable\nPress d to refresh dashboard".to_string(),
    };
    f.render_widget(
        Paragraph::new(text).block(Block::default().borders(Borders::ALL).title("Dashboard")),
        area,
    );
}
