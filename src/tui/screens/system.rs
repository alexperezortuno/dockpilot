use crate::{app::App, tui::Palette};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
};

pub fn draw(f: &mut Frame, app: &App, area: Rect, colors: Palette) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let Some(snapshot) = app.system_snapshot.as_ref() else {
        f.render_widget(
            Paragraph::new("Collecting local machine information...")
                .block(Block::default().borders(Borders::ALL).title("System")),
            area,
        );
        return;
    };

    let identity = vec![
        label("Operating system", snapshot.os.as_str(), colors),
        label("Kernel", snapshot.kernel.as_str(), colors),
        label("Hostname", snapshot.hostname.as_str(), colors),
        label("Architecture", snapshot.architecture.as_str(), colors),
        label("Uptime", format_duration(snapshot.uptime_seconds), colors),
        label("Dockpilot", snapshot.dockpilot_version.as_str(), colors),
        label("Process ID", snapshot.process_id.to_string(), colors),
        label("Working directory", snapshot.current_dir.as_str(), colors),
    ];
    f.render_widget(panel("Identity", identity), columns[0]);

    let resources = vec![
        label(
            "CPU",
            format!(
                "{} ({} logical cores, {:.1}% used)",
                snapshot.cpu_model, snapshot.cpu_count, snapshot.cpu_usage
            ),
            colors,
        ),
        label(
            "Load average",
            format!(
                "{:.2} / {:.2} / {:.2} (1/5/15 min), {} processes",
                snapshot.load_one,
                snapshot.load_five,
                snapshot.load_fifteen,
                snapshot.process_count
            ),
            colors,
        ),
        label(
            "Memory",
            format!(
                "{} used of {}",
                format_bytes(snapshot.memory_used),
                format_bytes(snapshot.memory_total)
            ),
            colors,
        ),
        label(
            "Swap",
            format!(
                "{} used of {}",
                format_bytes(snapshot.swap_used),
                format_bytes(snapshot.swap_total)
            ),
            colors,
        ),
        Line::from(Span::styled(
            "Disks",
            Style::default().add_modifier(Modifier::BOLD),
        )),
    ];
    let resources = snapshot.disks.iter().fold(resources, |mut lines, disk| {
        lines.push(label(
            &disk.mount_point,
            format!(
                "{} free of {} ({:.1}% available)",
                format_bytes(disk.available),
                format_bytes(disk.total),
                percentage(disk.available, disk.total)
            ),
            colors,
        ));
        lines
    });
    let resources = snapshot
        .networks
        .iter()
        .fold(resources, |mut lines, network| {
            lines.push(label(
                format!("Network {}", network.name),
                if network.addresses.is_empty() {
                    "no addresses".to_string()
                } else {
                    network.addresses.join(", ")
                },
                colors,
            ));
            lines
        });
    f.render_widget(panel("Resources", resources), columns[1]);
}

fn panel(title: &str, lines: Vec<Line<'static>>) -> Paragraph<'static> {
    Paragraph::new(Text::from(lines))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title.to_string()),
        )
        .wrap(Wrap { trim: true })
}

fn label(name: impl Into<String>, value: impl Into<String>, colors: Palette) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("{}: ", name.into()),
            Style::default()
                .fg(colors.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(value.into()),
    ])
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KiB", "MiB", "GiB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{} {}", bytes, UNITS[unit])
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

fn format_duration(seconds: u64) -> String {
    let days = seconds / 86_400;
    let hours = (seconds % 86_400) / 3_600;
    let minutes = (seconds % 3_600) / 60;
    format!("{days}d {hours}h {minutes}m")
}

fn percentage(value: u64, total: u64) -> f64 {
    if total == 0 {
        0.0
    } else {
        value as f64 / total as f64 * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::{format_bytes, format_duration, percentage};

    #[test]
    fn formats_system_values_for_humans() {
        assert_eq!(format_bytes(1024), "1.0 KiB");
        assert_eq!(format_duration(90_061), "1d 1h 1m");
        assert_eq!(percentage(25, 100), 25.0);
        assert_eq!(percentage(1, 0), 0.0);
    }
}
