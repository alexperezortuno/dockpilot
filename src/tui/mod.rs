pub mod terminal;

use crate::app::{
    App, ContainerAction, ImageAction, MachineAction, NetworkAction, ProjectAction, Tab,
    VolumeAction,
};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, List, ListItem, ListState, Paragraph, Row, Table, Tabs},
};

pub fn draw_app(f: &mut Frame, app: &mut App) {
    let size = f.area();

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
        "Container",
        "Image",
        "Network",
        "Volume",
        "Project",
        "Machine",
        "Help",
    ];

    let selected_tab = match app.current_tab {
        Tab::Container => 0,
        Tab::Image => 1,
        Tab::Network => 2,
        Tab::Volume => 3,
        Tab::Project => 4,
        Tab::Machine => 5,
        Tab::Help => 6,
    };

    let tabs = Tabs::new(tab_titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" Dockpilot | Docker: {} ", app.engine_status)),
        )
        .select(selected_tab)
        .style(Style::default().fg(Color::White))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );

    f.render_widget(tabs, chunks[0]);

    if matches!(app.current_tab, Tab::Container) {
        draw_container_tab(f, app, chunks[1]);
        draw_output(f, app, chunks[2]);
        if let Some(area) = input_area {
            draw_input(f, app, area);
        }
        return;
    }

    // List of actions
    let (items, title, state): (Vec<ListItem>, &str, &mut ListState) = match app.current_tab {
        Tab::Container => {
            let items = app
                .container_actions
                .iter()
                .map(|a| {
                    let s = match a {
                        ContainerAction::Start => "Start Containers",
                        ContainerAction::StopAll => "Stop all Containers",
                        ContainerAction::Stop => "Stop Containers",
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
                    };
                    ListItem::new(s)
                })
                .collect();
            (items, "Project Actions", &mut app.project_list_state)
        }
        Tab::Machine => {
            let items = app
                .machine_actions
                .iter()
                .map(|a| {
                    let s = match a {
                        MachineAction::List => "List Machines",
                        MachineAction::Start => "Start Machine",
                        MachineAction::Stop => "Stop Machine",
                        MachineAction::Env => "Show Machine Env",
                        MachineAction::Eval => "Eval Machine Env",
                        MachineAction::Ip => "Get Machine IP",
                    };
                    ListItem::new(s)
                })
                .collect();
            (items, "Machine Actions", &mut app.machine_list_state)
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
                "  q / Esc         - Exit",
                "",
                "Input mode:",
                "  Enter    - Confirm",
                "  Esc      - Cancel",
                "  Backspace- Delete",
                "  y / n    - Confirm or cancel pending action",
                "",
                "Available tabs:",
                "  Container, Image, Network, Volume, Project, Machine",
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
                .bg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    f.render_stateful_widget(list, chunks[1], state);

    draw_output(f, app, chunks[2]);

    if let Some(area) = input_area {
        draw_input(f, app, area);
    }
}

fn draw_container_tab(f: &mut Frame, app: &mut App, area: Rect) {
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
            .bg(Color::Blue)
            .add_modifier(Modifier::BOLD),
    )
    .highlight_symbol(">> ");
    f.render_stateful_widget(table, panes[0], &mut app.container_table_state);

    let items = app.container_actions.iter().map(|action| {
        let label = match action {
            ContainerAction::Start => "Start Containers",
            ContainerAction::StopAll => "Stop all Containers",
            ContainerAction::Stop => "Stop Containers",
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
                .bg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");
    let mut action_state = std::mem::take(&mut app.container_list_state);
    f.render_stateful_widget(list, panes[1], &mut action_state);
    app.container_list_state = action_state;
}

fn draw_output(f: &mut Frame, app: &mut App, area: Rect) {
    let visible_height = area.height.saturating_sub(2) as usize;
    let total = app.output_lines.len();
    let start = total.saturating_sub(visible_height);

    let items: Vec<ListItem> = app
        .output_lines
        .iter()
        .skip(start)
        .map(|l| ListItem::new(l.clone()))
        .collect();

    let output_list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" Output ({} líneas) ", total)),
    );

    f.render_widget(output_list, area);
}

fn draw_input(f: &mut Frame, app: &App, area: Rect) {
    let text = format!("{} {}", app.input_prompt, app.input_buffer);
    let input_widget = Paragraph::new(text).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Entrada (Enter=OK, Esc=Cancelar) ")
            .border_style(Style::default().fg(Color::Yellow)),
    );
    f.render_widget(input_widget, area);
}
