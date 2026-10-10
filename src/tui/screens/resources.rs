use crate::app::{
    App, ContainerAction, FocusTarget, ImageAction, NetworkAction, ProjectAction, Tab, VolumeAction,
};
use crate::tui::{
    Palette,
    widgets::{details, empty, navigation, table},
};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, Cell, List, ListItem, Row},
};

pub fn draw(f: &mut Frame, app: &mut App, area: Rect, colors: Palette) {
    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(72), Constraint::Percentage(28)])
        .split(area);
    match app.current_tab {
        Tab::Container => draw_containers(f, app, panes[0], panes[1], colors),
        Tab::Image => draw_images(f, app, panes[0], panes[1], colors),
        Tab::Network => draw_networks(f, app, panes[0], panes[1], colors),
        Tab::Volume => draw_volumes(f, app, panes[0], panes[1], colors),
        _ => unreachable!("resource renderer called for non-resource tab"),
    }
}

pub fn draw_actions(f: &mut Frame, app: &mut App, area: Rect, colors: Palette) {
    if matches!(app.current_tab, Tab::Help) {
        let help = [
            "Dockpilot TUI",
            "",
            "Controls:",
            "  Tab / Shift+Tab - Change focus",
            "  Left / Right    - Change resource tab",
            "  Up/Down / j/k   - Navigate focused component",
            "  Enter           - Execute / prompt for parameter",
            "  x               - Cancel active task",
            "  c               - Clear general output",
            "  PageUp/PageDown - Scroll output",
            "  Home/End        - Output start/end",
            "  f               - Filter current table",
            "  /               - Incremental resource search",
            "  :               - Command palette",
            "  a               - Selected resource actions",
            "  m               - Toggle table/actions focus",
            "  ?               - Open contextual help",
            "  q / Esc         - Exit",
            "",
            "Input mode:",
            "  Enter    - Confirm",
            "  Esc      - Cancel",
            "  Backspace- Delete",
            "  y / n    - Confirm or cancel pending action",
            "  Backspace- Dismiss notification",
        ];
        f.render_widget(
            List::new(help.into_iter().map(ListItem::new))
                .block(Block::default().borders(Borders::ALL).title("Help")),
            area,
        );
        return;
    }

    let (title, items) = match app.current_tab {
        Tab::Project => (
            "Project Actions",
            app.project_actions
                .iter()
                .map(|action| ListItem::new(project_label(action))),
        ),
        _ => unreachable!("action renderer called for non-action tab"),
    };
    table::draw_actions(f, area, title, items, &mut app.project_list_state, colors);
}

fn draw_containers(
    f: &mut Frame,
    app: &mut App,
    table_area: Rect,
    details_area: Rect,
    colors: Palette,
) {
    let containers = app.filtered_containers();
    let rows = containers.iter().map(|container| {
        Row::new(vec![
            Cell::from(container.id.chars().take(12).collect::<String>()),
            Cell::from(container.name.clone()),
            Cell::from(container.image.clone()),
            Cell::from(container.state.clone()),
            Cell::from(container.status.clone()),
        ])
    });
    table::draw_table(
        f,
        table_area,
        rows,
        [
            Constraint::Length(12),
            Constraint::Min(16),
            Constraint::Min(16),
            Constraint::Length(12),
            Constraint::Min(20),
        ],
        ["ID", "Name", "Image", "State", "Status"]
            .into_iter()
            .map(Cell::from),
        navigation::title(
            "Containers",
            containers.len(),
            &app.container_filter,
            app.focus_target,
        ),
        &mut app.container_table_state,
        colors,
    );
    if app.focus_target == FocusTarget::Table {
        let text = app
            .container_table_state
            .selected()
            .and_then(|index| containers.get(index))
            .map(|container| {
                format!(
                    "Name: {}\nImage: {}\nState: {}\nStatus: {}",
                    container.name, container.image, container.state, container.status
                )
            })
            .unwrap_or_else(|| empty::message("containers", 'r'));
        details::draw(f, details_area, "Container Details", text);
    } else {
        table::draw_actions(
            f,
            details_area,
            "Actions",
            app.container_actions
                .iter()
                .map(|action| ListItem::new(container_label(action))),
            &mut app.container_list_state,
            colors,
        );
    }
}

fn draw_images(
    f: &mut Frame,
    app: &mut App,
    table_area: Rect,
    details_area: Rect,
    colors: Palette,
) {
    let images = app.filtered_images();
    let rows = images.iter().map(|image| {
        Row::new(vec![
            Cell::from(image.id.clone()),
            Cell::from(image.tag.clone()),
            Cell::from(format!("{} MB", image.size / 1_048_576)),
        ])
    });
    table::draw_table(
        f,
        table_area,
        rows,
        [
            Constraint::Length(14),
            Constraint::Min(24),
            Constraint::Length(12),
        ],
        ["ID", "Tag", "Size"].into_iter().map(Cell::from),
        navigation::title("Images", images.len(), &app.image_filter, app.focus_target),
        &mut app.image_table_state,
        colors,
    );
    if app.focus_target == FocusTarget::Table {
        let text = app
            .image_table_state
            .selected()
            .and_then(|index| images.get(index))
            .map(|image| {
                format!(
                    "ID: {}\nTag: {}\nSize: {} bytes",
                    image.id, image.tag, image.size
                )
            })
            .unwrap_or_else(|| empty::message("images", 'i'));
        details::draw(f, details_area, "Image Details", text);
    } else {
        table::draw_actions(
            f,
            details_area,
            "Image Actions",
            app.image_actions
                .iter()
                .map(|action| ListItem::new(image_label(action))),
            &mut app.image_list_state,
            colors,
        );
    }
}

fn draw_networks(
    f: &mut Frame,
    app: &mut App,
    table_area: Rect,
    details_area: Rect,
    colors: Palette,
) {
    let networks = app.filtered_networks();
    let rows = networks.iter().map(|network| {
        Row::new(vec![
            Cell::from(network.id.clone()),
            Cell::from(network.name.clone()),
            Cell::from(network.driver.clone()),
            Cell::from(network.scope.clone()),
        ])
    });
    table::draw_table(
        f,
        table_area,
        rows,
        [
            Constraint::Length(12),
            Constraint::Min(20),
            Constraint::Length(12),
            Constraint::Length(12),
        ],
        ["ID", "Name", "Driver", "Scope"]
            .into_iter()
            .map(Cell::from),
        navigation::title(
            "Networks",
            networks.len(),
            &app.network_filter,
            app.focus_target,
        ),
        &mut app.network_table_state,
        colors,
    );
    if app.focus_target == FocusTarget::Table {
        let text = app
            .network_table_state
            .selected()
            .and_then(|index| networks.get(index))
            .map(|network| {
                format!(
                    "Name: {}\nDriver: {}\nScope: {}",
                    network.name, network.driver, network.scope
                )
            })
            .unwrap_or_else(|| empty::message("networks", 'n'));
        details::draw(f, details_area, "Network Details", text);
    } else {
        table::draw_actions(
            f,
            details_area,
            "Actions",
            app.network_actions
                .iter()
                .map(|action| ListItem::new(network_label(action))),
            &mut app.network_list_state,
            colors,
        );
    }
}

fn draw_volumes(
    f: &mut Frame,
    app: &mut App,
    table_area: Rect,
    details_area: Rect,
    colors: Palette,
) {
    let volumes = app.filtered_volumes();
    let rows = volumes.iter().map(|volume| {
        Row::new(vec![
            Cell::from(volume.name.clone()),
            Cell::from(volume.driver.clone()),
            Cell::from(volume.mountpoint.clone()),
        ])
    });
    table::draw_table(
        f,
        table_area,
        rows,
        [
            Constraint::Min(24),
            Constraint::Length(16),
            Constraint::Min(36),
        ],
        ["Name", "Driver", "Mountpoint"].into_iter().map(Cell::from),
        navigation::title(
            "Volumes",
            volumes.len(),
            &app.volume_filter,
            app.focus_target,
        ),
        &mut app.volume_table_state,
        colors,
    );
    if app.focus_target == FocusTarget::Table {
        let text = app
            .volume_table_state
            .selected()
            .and_then(|index| volumes.get(index))
            .map(|volume| {
                format!(
                    "Name: {}\nDriver: {}\nMountpoint: {}",
                    volume.name, volume.driver, volume.mountpoint
                )
            })
            .unwrap_or_else(|| empty::message("volumes", 'v'));
        details::draw(f, details_area, "Volume Details", text);
    } else {
        table::draw_actions(
            f,
            details_area,
            "Actions",
            app.volume_actions
                .iter()
                .map(|action| ListItem::new(volume_label(action))),
            &mut app.volume_list_state,
            colors,
        );
    }
}

fn container_label(action: &ContainerAction) -> &'static str {
    match action {
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
    }
}

fn image_label(action: &ImageAction) -> &'static str {
    match action {
        ImageAction::Build => "Build Image",
        ImageAction::Rebuild => "Rebuild Image (no-cache)",
        ImageAction::List => "Refresh Images",
        ImageAction::Remove => "Remove Image",
        ImageAction::Push => "Push Image",
        ImageAction::Pull => "Pull Image",
        ImageAction::Save => "Save Image to tar",
        ImageAction::Load => "Load Image from tar",
        ImageAction::History => "View Image History",
    }
}

fn network_label(action: &NetworkAction) -> &'static str {
    match action {
        NetworkAction::List => "Refresh Networks",
        NetworkAction::Create => "Create Network",
        NetworkAction::Remove => "Remove Network",
    }
}

fn volume_label(action: &VolumeAction) -> &'static str {
    match action {
        VolumeAction::List => "Refresh Volumes",
        VolumeAction::Create => "Create Volume",
        VolumeAction::Remove => "Remove Volume",
        VolumeAction::Backup => "Backup Volume",
        VolumeAction::Restore => "Restore Volume",
    }
}

fn project_label(action: &ProjectAction) -> &'static str {
    match action {
        ProjectAction::SetFolder => "Set Project Folder",
        ProjectAction::ComposeUp => "Compose Up",
        ProjectAction::ComposeUpProfile => "Compose Up (Profile)",
        ProjectAction::ComposeDown => "Compose Down",
        ProjectAction::ComposeConfig => "Compose Config",
    }
}
