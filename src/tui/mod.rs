pub mod terminal;

use crate::app::{
    App, ContainerAction, ImageAction, NetworkAction, ProjectAction, Tab, VolumeAction,
};
use crate::config::ThemeName;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, List, ListItem, ListState, Paragraph, Row, Table, Tabs},
};

#[derive(Clone, Copy)]
struct Palette {
    foreground: Color,
    accent: Color,
    selection: Color,
}

fn palette(theme: ThemeName) -> Palette {
    match theme {
        ThemeName::Dark => Palette {
            foreground: Color::White,
            accent: Color::Yellow,
            selection: Color::Blue,
        },
        ThemeName::Light => Palette {
            foreground: Color::Black,
            accent: Color::Green,
            selection: Color::LightBlue,
        },
        ThemeName::Mono => Palette {
            foreground: Color::Gray,
            accent: Color::White,
            selection: Color::DarkGray,
        },
    }
}

pub fn draw_app(f: &mut Frame, app: &mut App) {
    let size = f.area();
    let colors = palette(app.theme);

    // If we are in input mode, we reserve 3 lines at the bottom for the prompt.
    let (main_area, input_area) = if app.input_mode {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(10), Constraint::Length(3)])
            .split(size);
        (chunks[0], Some(chunks[1]))
    } else {
        (size, None)
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(15),
            Constraint::Min(10),
        ])
        .split(main_area);

    // Tabs
    let tab_titles = [
        "Dashboard",
        "Container",
        "Image",
        "Network",
        "Volume",
        "Project",
        "Help",
    ];

    let selected_tab = match app.current_tab {
        Tab::Dashboard => 0,
        Tab::Container => 1,
        Tab::Image => 2,
        Tab::Network => 3,
        Tab::Volume => 4,
        Tab::Project => 5,
        Tab::Help => 6,
    };

    let tabs = Tabs::new(tab_titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" Dockpilot | Docker: {} ", app.engine_status)),
        )
        .select(selected_tab)
        .style(Style::default().fg(colors.foreground))
        .highlight_style(
            Style::default()
                .fg(colors.accent)
                .add_modifier(Modifier::BOLD),
        );

    f.render_widget(tabs, chunks[0]);

    if matches!(app.current_tab, Tab::Dashboard) {
        draw_dashboard(f, app, chunks[1]);
        draw_output(f, app, chunks[2]);
        return;
    }

    if matches!(app.current_tab, Tab::Image) {
        draw_image_tab(f, app, chunks[1], colors);
        draw_output(f, app, chunks[2]);
        if let Some(area) = input_area {
            draw_input(f, app, area);
        }
        return;
    }

    if matches!(app.current_tab, Tab::Network) {
        draw_network_tab(f, app, chunks[1], colors);
        draw_output(f, app, chunks[2]);
        if let Some(area) = input_area {
            draw_input(f, app, area);
        }
        return;
    }

    if matches!(app.current_tab, Tab::Volume) {
        draw_volume_tab(f, app, chunks[1], colors);
        draw_output(f, app, chunks[2]);
        if let Some(area) = input_area {
            draw_input(f, app, area);
        }
        return;
    }

    if matches!(app.current_tab, Tab::Container) {
        draw_container_tab(f, app, chunks[1], colors);
        draw_output(f, app, chunks[2]);
        if let Some(area) = input_area {
            draw_input(f, app, area);
        }
        return;
    }

    // List of actions
    let (items, title, state): (Vec<ListItem>, &str, &mut ListState) = match app.current_tab {
        Tab::Dashboard => (Vec::new(), "Dashboard", &mut app.container_list_state),
        Tab::Container => {
            let items = app
                .container_actions
                .iter()
                .map(|a| {
                    let s = match a {
                        ContainerAction::StartAll => "Start All Containers",
                        ContainerAction::Start => "Start Container by Name",
                        ContainerAction::StopAll => "Stop all Containers",
                        ContainerAction::Stop => "Stop Container by Name",
                        ContainerAction::Restart => "Restart Container",
                        ContainerAction::ListAll => "List All Containers",
                        ContainerAction::List => "List Containers",
                        ContainerAction::Logs => "View Container Logs",
                        ContainerAction::Create => "Create Container",
                        ContainerAction::Remove => "Remove Container",
                        ContainerAction::Top => "View Container Top",
                        ContainerAction::Diff => "View Container Diff",
                        ContainerAction::Pause => "Pause Container",
                        ContainerAction::Unpause => "Unpause Container",
                        ContainerAction::Update => "Update Container",
                        ContainerAction::Wait => "Wait for Container",
                    };
                    ListItem::new(s)
                })
                .collect();
            (items, "Container Actions", &mut app.container_list_state)
        }
        Tab::Image => {
            let items = app
                .image_actions
                .iter()
                .map(|a| {
                    let s = match a {
                        ImageAction::Build => "Build Image",
                        ImageAction::Rebuild => "Rebuild Image (no-cache)",
                        ImageAction::List => "List Images",
                        ImageAction::Remove => "Remove Image",
                        ImageAction::Push => "Push Image",
                        ImageAction::Pull => "Pull Image",
                        ImageAction::Save => "Save Image to tar",
                        ImageAction::Load => "Load Image from tar",
                        ImageAction::History => "View Image History",
                    };
                    ListItem::new(s)
                })
                .collect();
            (items, "Image Actions", &mut app.image_list_state)
        }
        Tab::Network => {
            let items = app
                .network_actions
                .iter()
                .map(|a| {
                    let s = match a {
                        NetworkAction::List => "List Networks",
                        NetworkAction::Create => "Create Network",
                        NetworkAction::Remove => "Remove Network",
                    };
                    ListItem::new(s)
                })
                .collect();
            (items, "Network Actions", &mut app.network_list_state)
        }
        Tab::Volume => {
            let items = app
                .volume_actions
                .iter()
                .map(|a| {
                    let s = match a {
                        VolumeAction::List => "List Volumes",
                        VolumeAction::Create => "Create Volume",
                        VolumeAction::Remove => "Remove Volume",
                    };
                    ListItem::new(s)
                })
                .collect();
            (items, "Volume Actions", &mut app.volume_list_state)
        }
        Tab::Project => {
            let items = app
                .project_actions
                .iter()
                .map(|a| {
                    let s = match a {
                        ProjectAction::SetFolder => "Set Project Folder",
                        ProjectAction::ComposeUp => "Compose Up",
                        ProjectAction::ComposeUpProfile => "Compose Up (Profile)",
                        ProjectAction::ComposeDown => "Compose Down",
                        ProjectAction::ComposeConfig => "Compose Config",
                    };
                    ListItem::new(s)
                })
                .collect();
            (items, "Project Actions", &mut app.project_list_state)
        }
        Tab::Help => {
            let help = vec![
                "Dockpilot TUI",
                "",
                "Controls:",
                "  Tab / Shift+Tab - Switch tab",
                "  Up/Down         - Navigate actions",
                "  Enter           - Execute / prompt for parameter",
                "  x               - Cancel active task",
                "  r               - Refresh containers",
                "  f               - Filter containers",
                "  m               - Toggle table/actions focus",
                "  s               - Cycle container sort",
                "  p               - Pause/resume log display",
                "  /               - Filter log lines",
                "  q / Esc         - Exit",
                "",
                "Input mode:",
                "  Enter    - Confirm",
                "  Esc      - Cancel",
                "  Backspace- Delete",
                "  y / n    - Confirm or cancel pending action",
                "",
                "Available tabs:",
                "  Dashboard, Container, Image, Network, Volume, Project",
            ];
            let items: Vec<ListItem> = help.iter().map(|l| ListItem::new(*l)).collect();
            let list = List::new(items).block(Block::default().borders(Borders::ALL).title("Help"));
            f.render_widget(list, chunks[1]);
            draw_output(f, app, chunks[2]);
            if let Some(area) = input_area {
                draw_input(f, app, area);
            }
            return;
        }
    };

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(
            Style::default()
                .bg(colors.selection)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    f.render_stateful_widget(list, chunks[1], state);

    draw_output(f, app, chunks[2]);

    if let Some(area) = input_area {
        draw_input(f, app, area);
    }
}

fn draw_dashboard(f: &mut Frame, app: &App, area: Rect) {
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
    let widget =
        Paragraph::new(text).block(Block::default().borders(Borders::ALL).title("Dashboard"));
    f.render_widget(widget, area);
}

fn draw_image_tab(f: &mut Frame, app: &mut App, area: Rect, colors: Palette) {
    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(72), Constraint::Percentage(28)])
        .split(area);
    let rows = app.images.iter().map(|image| {
        Row::new([
            Cell::from(image.id.clone()),
            Cell::from(image.tag.clone()),
            Cell::from(format!("{} MB", image.size / 1_048_576)),
        ])
    });
    let table = Table::new(
        rows,
        [
            Constraint::Length(14),
            Constraint::Min(24),
            Constraint::Length(12),
        ],
    )
    .header(Row::new(["ID", "Tag", "Size"]))
    .block(Block::default().borders(Borders::ALL).title(format!(
        " Images | focus: {} ",
        if app.image_table_focus {
            "table"
        } else {
            "actions"
        }
    )))
    .row_highlight_style(
        Style::default()
            .bg(colors.selection)
            .add_modifier(Modifier::BOLD),
    )
    .highlight_symbol(">> ");
    f.render_stateful_widget(table, panes[0], &mut app.image_table_state);

    let items = app.image_actions.iter().map(|action| {
        let label = match action {
            ImageAction::Build => "Build Image",
            ImageAction::Rebuild => "Rebuild Image (no-cache)",
            ImageAction::List => "Refresh Images",
            ImageAction::Remove => "Remove Image",
            ImageAction::Push => "Push Image",
            ImageAction::Pull => "Pull Image",
            ImageAction::Save => "Save Image to tar",
            ImageAction::Load => "Load Image from tar",
            ImageAction::History => "View Image History",
        };
        ListItem::new(label)
    });
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Image Actions"),
        )
        .highlight_style(
            Style::default()
                .bg(colors.selection)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");
    f.render_stateful_widget(list, panes[1], &mut app.image_list_state);
}

fn draw_network_tab(f: &mut Frame, app: &mut App, area: Rect, colors: Palette) {
    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(72), Constraint::Percentage(28)])
        .split(area);
    let rows = app.networks.iter().map(|network| {
        Row::new([
            network.id.clone(),
            network.name.clone(),
            network.driver.clone(),
            network.scope.clone(),
        ])
    });
    let table = Table::new(
        rows,
        [
            Constraint::Length(12),
            Constraint::Min(20),
            Constraint::Length(12),
            Constraint::Length(12),
        ],
    )
    .header(Row::new(["ID", "Name", "Driver", "Scope"]))
    .block(Block::default().borders(Borders::ALL).title(format!(
        " Networks | focus: {} ",
        if app.network_table_focus {
            "table"
        } else {
            "actions"
        }
    )))
    .row_highlight_style(
        Style::default()
            .bg(colors.selection)
            .add_modifier(Modifier::BOLD),
    )
    .highlight_symbol(">> ");
    f.render_stateful_widget(table, panes[0], &mut app.network_table_state);
    let items = app.network_actions.iter().map(|action| {
        ListItem::new(match action {
            NetworkAction::List => "Refresh Networks",
            NetworkAction::Create => "Create Network",
            NetworkAction::Remove => "Remove Network",
        })
    });
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Actions"))
        .highlight_style(
            Style::default()
                .bg(colors.selection)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");
    f.render_stateful_widget(list, panes[1], &mut app.network_list_state);
}

fn draw_volume_tab(f: &mut Frame, app: &mut App, area: Rect, colors: Palette) {
    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(72), Constraint::Percentage(28)])
        .split(area);
    let rows = app.volumes.iter().map(|volume| {
        Row::new([
            volume.name.clone(),
            volume.driver.clone(),
            volume.mountpoint.clone(),
        ])
    });
    let table = Table::new(
        rows,
        [
            Constraint::Min(24),
            Constraint::Length(16),
            Constraint::Min(36),
        ],
    )
    .header(Row::new(["Name", "Driver", "Mountpoint"]))
    .block(Block::default().borders(Borders::ALL).title(format!(
        " Volumes | focus: {} ",
        if app.volume_table_focus {
            "table"
        } else {
            "actions"
        }
    )))
    .row_highlight_style(
        Style::default()
            .bg(colors.selection)
            .add_modifier(Modifier::BOLD),
    )
    .highlight_symbol(">> ");
    f.render_stateful_widget(table, panes[0], &mut app.volume_table_state);
    let items = app.volume_actions.iter().map(|action| {
        ListItem::new(match action {
            VolumeAction::List => "Refresh Volumes",
            VolumeAction::Create => "Create Volume",
            VolumeAction::Remove => "Remove Volume",
        })
    });
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Actions"))
        .highlight_style(
            Style::default()
                .bg(colors.selection)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");
    f.render_stateful_widget(list, panes[1], &mut app.volume_list_state);
}

fn draw_container_tab(f: &mut Frame, app: &mut App, area: Rect, colors: Palette) {
    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(72), Constraint::Percentage(28)])
        .split(area);
    let containers = app.filtered_containers();
    let rows = containers.iter().map(|container| {
        Row::new([
            Cell::from(container.id.chars().take(12).collect::<String>()),
            Cell::from(container.name.clone()),
            Cell::from(container.image.clone()),
            Cell::from(container.state.clone()),
            Cell::from(container.status.clone()),
        ])
    });
    let table = Table::new(
        rows,
        [
            Constraint::Length(12),
            Constraint::Min(16),
            Constraint::Min(16),
            Constraint::Length(12),
            Constraint::Min(20),
        ],
    )
    .header(Row::new(["ID", "Name", "Image", "State", "Status"]))
    .block(Block::default().borders(Borders::ALL).title(format!(
        " Containers ({}) | filter: {} | focus: {} ",
        containers.len(),
        if app.container_filter.is_empty() {
            "none"
        } else {
            &app.container_filter
        },
        if app.container_table_focus {
            "table"
        } else {
            "actions"
        }
    )))
    .row_highlight_style(
        Style::default()
            .bg(colors.selection)
            .add_modifier(Modifier::BOLD),
    )
    .highlight_symbol(">> ");
    f.render_stateful_widget(table, panes[0], &mut app.container_table_state);

    let items = app.container_actions.iter().map(|action| {
        let label = match action {
            ContainerAction::StartAll => "Start All Containers",
            ContainerAction::Start => "Start Container by Name",
            ContainerAction::StopAll => "Stop all Containers",
            ContainerAction::Stop => "Stop Container by Name",
            ContainerAction::Restart => "Restart Container",
            ContainerAction::ListAll => "List All Containers",
            ContainerAction::List => "List Containers",
            ContainerAction::Logs => "View Container Logs",
            ContainerAction::Create => "Create Container",
            ContainerAction::Remove => "Remove Container",
            ContainerAction::Top => "View Container Top",
            ContainerAction::Diff => "View Container Diff",
            ContainerAction::Pause => "Pause Container",
            ContainerAction::Unpause => "Unpause Container",
            ContainerAction::Update => "Update Container",
            ContainerAction::Wait => "Wait for Container",
        };
        ListItem::new(label)
    });
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Actions"))
        .highlight_style(
            Style::default()
                .bg(colors.selection)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");
    let mut action_state = std::mem::take(&mut app.container_list_state);
    f.render_stateful_widget(list, panes[1], &mut action_state);
    app.container_list_state = action_state;
}

fn draw_output(f: &mut Frame, app: &mut App, area: Rect) {
    let visible_height = area.height.saturating_sub(2) as usize;
    let showing_logs = !app.log_lines.is_empty();
    let lines = if showing_logs {
        app.filtered_log_lines()
    } else {
        app.output_lines.iter().cloned().collect()
    };
    let total = lines.len();
    let start = total.saturating_sub(visible_height);

    let items: Vec<ListItem> = lines
        .iter()
        .skip(start)
        .map(|l| ListItem::new(l.clone()))
        .collect();

    let output_list = List::new(items).block(Block::default().borders(Borders::ALL).title(
        if showing_logs {
            format!(
                " Logs ({} lines{}) ",
                total,
                if app.logs_paused { ", paused" } else { "" }
            )
        } else {
            format!(" Output ({} lines) ", total)
        },
    ));

    f.render_widget(output_list, area);
}

fn draw_input(f: &mut Frame, app: &App, area: Rect) {
    let text = format!("{} {}", app.input_prompt, app.input_buffer);
    let input_widget = Paragraph::new(text).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Entry (Enter=OK, Esc=Cancel) ")
            .border_style(Style::default().fg(Color::Yellow)),
    );
    f.render_widget(input_widget, area);
}
