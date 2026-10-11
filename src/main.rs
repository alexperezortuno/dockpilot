mod app;
mod config;
mod docker;
mod security;
mod system;
mod tasks;
mod tui;

use app::task_history::TaskState;
use app::{App, Overlay, Tab, notifications::NotificationKind};
use clap::Parser;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use docker::client::{
    EngineConnection, dashboard_data, list_containers, list_images, list_networks, list_volumes,
};
use ratatui::{Terminal, backend::CrosstermBackend};
use security::SafetyPolicy;
use std::{
    collections::HashMap,
    io,
    time::{Duration, Instant},
};
use tasks::{TaskEvent, TaskManager, TaskOrigin, TaskRequest};
use tui::terminal::TerminalGuard;

async fn run_json(
    config: &config::Config,
    cli: &config::Cli,
) -> Result<(), Box<dyn std::error::Error>> {
    let connection = EngineConnection::connect(config.docker_context.as_deref()).await;
    let status = connection.status_message();
    let client = match connection.into_client() {
        Some(client) => client,
        None => {
            println!("{}", serde_json::json!({ "error": status }));
            return Ok(());
        }
    };

    let value = if cli.list_containers {
        match list_containers(&client).await {
            Ok(value) => serde_json::json!(value),
            Err(error) => serde_json::json!({ "error": error }),
        }
    } else if cli.list_images {
        match list_images(&client).await {
            Ok(value) => serde_json::json!(value),
            Err(error) => serde_json::json!({ "error": error }),
        }
    } else if cli.list_networks {
        match list_networks(&client).await {
            Ok(value) => serde_json::json!(value),
            Err(error) => serde_json::json!({ "error": error }),
        }
    } else if cli.list_volumes {
        match list_volumes(&client).await {
            Ok(value) => serde_json::json!(value),
            Err(error) => serde_json::json!({ "error": error }),
        }
    } else if cli.dashboard {
        match dashboard_data(&client, None).await {
            Ok(value) => serde_json::json!(value),
            Err(error) => serde_json::json!({ "error": error }),
        }
    } else {
        serde_json::json!({
            "error": "select one of --list-containers, --list-images, --list-networks, --list-volumes, or --dashboard"
        })
    };
    println!("{}", serde_json::to_string_pretty(&value)?);
    Ok(())
}

fn dispatch_request(
    app: &mut App,
    task_manager: &mut TaskManager,
    policy: SafetyPolicy,
    pending_confirmation: &mut Option<TaskRequest>,
    request: TaskRequest,
    confirmed: bool,
) {
    let mutation = request.mutation();
    let description = request.description();
    if !policy.allows(mutation) {
        app.push_output(format!("[blocked: read-only context: {}]", description));
        app.notify(
            NotificationKind::Warning,
            "Action blocked by read-only policy",
            true,
        );
        return;
    }
    if !confirmed && policy.requires_confirmation(mutation) {
        let consequence = if mutation == security::Mutation::Destructive {
            "; consequence: the resource will be removed"
        } else {
            ""
        };
        app.push_output(format!(
            "[confirmation required: {}{} (y/n)]",
            description, consequence
        ));
        app.notify(
            NotificationKind::Warning,
            format!("Confirm: {}{}", description, consequence),
            true,
        );
        *pending_confirmation = Some(request);
        return;
    }
    if !task_manager.spawn(request) {
        app.push_output("[Task in progress: press x to cancel]");
        app.notify(
            NotificationKind::Warning,
            "A task is already in progress",
            false,
        );
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = config::Cli::parse();
    let mut config = config::Config::load(&cli)?;
    if cli.json {
        return run_json(&config, &cli).await;
    }
    let policy = SafetyPolicy::new(config.safe_mode, config.read_only);
    let shortcuts = config.shortcuts;
    let _terminal_guard = TerminalGuard::new()?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::with_project_folder(config.project_folder.clone());
    app.set_policy(policy);
    app.set_output_capacity(config.output_capacity);
    app.set_theme(config.theme);
    app.set_preferences(
        config.auto_refresh,
        config.show_output,
        config.show_details,
        config.compact_layout,
        config.mouse_enabled,
    );
    let engine_connection = EngineConnection::connect(config.docker_context.as_deref()).await;
    let engine_status = engine_connection.status_message();
    let engine_status_display = engine_connection.status().to_string();
    let engine_client = engine_connection.into_client();
    app.set_engine_status(engine_status);
    app.push_output(format!("[docker] {}", engine_status_display));
    let mut task_manager = TaskManager::new(32);
    task_manager.set_client(engine_client);
    task_manager.spawn_with_origin(TaskRequest::SystemInfo, TaskOrigin::System);
    let poll_interval = Duration::from_millis(config.poll_interval_ms);
    let mut pending_confirmation: Option<TaskRequest> = None;
    let mut should_quit = false;
    let mut last_metrics = Instant::now();
    let mut last_health = Instant::now();
    let mut metrics_container: Option<String> = None;
    let mut last_container_refresh = Instant::now();
    let mut last_image_refresh = Instant::now();
    let mut last_network_refresh = Instant::now();
    let mut last_volume_refresh = Instant::now();
    let mut last_system_refresh = Instant::now();
    let mut task_origins = HashMap::new();
    let mut background_cursor = 0usize;
    let mut last_draw = Instant::now() - Duration::from_millis(250);

    while !should_quit {
        app.tick();
        while let Some(task_event) = task_manager.try_next() {
            match task_event {
                TaskEvent::Started {
                    id,
                    description,
                    origin,
                } => {
                    task_origins.insert(id, (origin, description.clone()));
                    app.queue_task(id, description);
                    app.start_task(id);
                    app.set_task_status("running");
                }
                TaskEvent::Progress { id, message } => {
                    app.set_task_status(format!("task {}: {}", id, message));
                }
                TaskEvent::Finished { id, lines } => {
                    let (origin, _) = task_origins
                        .remove(&id)
                        .unwrap_or((TaskOrigin::User, String::new()));
                    app.finish_task(id, TaskState::Completed);
                    app.set_task_status("idle");
                    if origin == TaskOrigin::User {
                        app.append_output(lines);
                        app.notify(
                            NotificationKind::Success,
                            format!("Task {} completed", id),
                            false,
                        );
                    }
                    task_manager.complete(id);
                }
                TaskEvent::Failed { id, message } => {
                    let (origin, description) = task_origins
                        .remove(&id)
                        .unwrap_or((TaskOrigin::User, String::new()));
                    app.finish_task(id, TaskState::Failed);
                    app.set_task_status("idle");
                    if origin == TaskOrigin::Background {
                        if app.refresh_failed(&description) {
                            app.notify(
                                NotificationKind::Error,
                                format!("{} failed: {}", description, message),
                                true,
                            );
                        }
                    } else {
                        app.push_output(format!("[docker] {}", message));
                        app.notify(
                            NotificationKind::Error,
                            format!("Task {} failed: {}", id, message),
                            true,
                        );
                    }
                    task_manager.complete(id);
                }
                TaskEvent::Cancelled { id } => {
                    task_origins.remove(&id);
                    app.finish_task(id, TaskState::Cancelled);
                    app.set_task_status("idle");
                    app.notify(
                        NotificationKind::Info,
                        format!("Task {} cancelled", id),
                        false,
                    );
                    app.mark_dirty();
                }
                TaskEvent::Containers { id, containers } => {
                    let (origin, description) = task_origins
                        .remove(&id)
                        .unwrap_or((TaskOrigin::User, String::new()));
                    app.set_task_status("idle");
                    let count = containers.len();
                    app.set_containers(containers);
                    if origin == TaskOrigin::User {
                        app.push_output(format!("[docker] loaded {} containers", count));
                        app.notify(NotificationKind::Success, "Containers refreshed", false);
                    } else if app.refresh_recovered(&description) {
                        app.notify(NotificationKind::Info, "Docker refresh recovered", false);
                    }
                    task_manager.complete(id);
                }
                TaskEvent::LogLine { line } => {
                    app.push_log_line(line);
                }
                TaskEvent::EventLine { line, alert } => {
                    app.set_task_status("events");
                    app.push_event(line, alert);
                }
                TaskEvent::Dashboard { id, data } => {
                    let (origin, description) = task_origins
                        .remove(&id)
                        .unwrap_or((TaskOrigin::User, String::new()));
                    app.set_task_status("idle");
                    app.set_dashboard(data);
                    if origin == TaskOrigin::Background && app.refresh_recovered(&description) {
                        app.notify(NotificationKind::Info, "Docker refresh recovered", false);
                    }
                    task_manager.complete(id);
                }
                TaskEvent::Metrics {
                    id,
                    container_id,
                    sample,
                } => {
                    let (origin, description) = task_origins
                        .remove(&id)
                        .unwrap_or((TaskOrigin::User, String::new()));
                    app.record_metrics(Instant::now(), sample);
                    if origin == TaskOrigin::Background {
                        app.set_task_status("metrics updated");
                    } else {
                        app.set_task_status(format!("metrics: {}", container_id));
                    }
                    if origin == TaskOrigin::Background {
                        app.refresh_recovered(&description);
                    }
                    task_manager.complete(id);
                }
                TaskEvent::Health { id, snapshot } => {
                    let (_, description) = task_origins
                        .remove(&id)
                        .unwrap_or((TaskOrigin::Background, String::new()));
                    app.set_health(snapshot);
                    app.set_task_status("health updated");
                    app.refresh_recovered(&description);
                    task_manager.complete(id);
                }
                TaskEvent::Images { id, images } => {
                    let (origin, description) = task_origins
                        .remove(&id)
                        .unwrap_or((TaskOrigin::User, String::new()));
                    app.set_task_status("idle");
                    let count = images.len();
                    app.set_images(images);
                    if origin == TaskOrigin::User {
                        app.push_output(format!("[docker] loaded {} images", count));
                        app.notify(NotificationKind::Success, "Images refreshed", false);
                    } else if app.refresh_recovered(&description) {
                        app.notify(NotificationKind::Info, "Docker refresh recovered", false);
                    }
                    task_manager.complete(id);
                }
                TaskEvent::Networks { id, networks } => {
                    let (origin, description) = task_origins
                        .remove(&id)
                        .unwrap_or((TaskOrigin::User, String::new()));
                    app.set_task_status("idle");
                    app.set_networks(networks);
                    if origin == TaskOrigin::Background && app.refresh_recovered(&description) {
                        app.notify(NotificationKind::Info, "Docker refresh recovered", false);
                    }
                    task_manager.complete(id);
                }
                TaskEvent::Volumes { id, volumes } => {
                    let (origin, description) = task_origins
                        .remove(&id)
                        .unwrap_or((TaskOrigin::User, String::new()));
                    app.set_task_status("idle");
                    app.set_volumes(volumes);
                    if origin == TaskOrigin::Background && app.refresh_recovered(&description) {
                        app.notify(NotificationKind::Info, "Docker refresh recovered", false);
                    }
                    task_manager.complete(id);
                }
                TaskEvent::System { id, snapshot } => {
                    let (origin, description) = task_origins
                        .remove(&id)
                        .unwrap_or((TaskOrigin::User, String::new()));
                    let load_containers = origin == TaskOrigin::System && task_manager.has_client();
                    app.set_system_snapshot(snapshot);
                    app.set_task_status("idle");
                    if origin == TaskOrigin::Background && app.refresh_recovered(&description) {
                        app.notify(NotificationKind::Info, "System refresh recovered", false);
                    }
                    task_manager.complete(id);
                    if load_containers {
                        task_manager
                            .spawn_with_origin(TaskRequest::ListContainers, TaskOrigin::System);
                    }
                }
            }
        }

        if task_manager.is_idle()
            && app.auto_refresh
            && last_system_refresh.elapsed()
                >= Duration::from_millis(config.poll_interval_ms.max(1000))
        {
            last_system_refresh = Instant::now();
            task_manager.spawn_with_origin(TaskRequest::SystemInfo, TaskOrigin::Background);
        }

        if task_manager.has_client() && task_manager.is_idle() && app.auto_refresh {
            for offset in 0..6 {
                let slot = (background_cursor + offset) % 6;
                let now = Instant::now();
                let request = match slot {
                    0 if last_container_refresh.elapsed()
                        >= Duration::from_millis(config.container_refresh_interval_ms) =>
                    {
                        last_container_refresh = now;
                        Some(TaskRequest::ListContainers)
                    }
                    1 if last_image_refresh.elapsed()
                        >= Duration::from_millis(config.image_refresh_interval_ms) =>
                    {
                        last_image_refresh = now;
                        Some(TaskRequest::ListImages)
                    }
                    2 if last_network_refresh.elapsed()
                        >= Duration::from_millis(config.network_refresh_interval_ms) =>
                    {
                        last_network_refresh = now;
                        Some(TaskRequest::ListNetworks)
                    }
                    3 if last_volume_refresh.elapsed()
                        >= Duration::from_millis(config.volume_refresh_interval_ms) =>
                    {
                        last_volume_refresh = now;
                        Some(TaskRequest::ListVolumes)
                    }
                    4 if last_metrics.elapsed()
                        >= Duration::from_millis(config.metrics_interval_ms) =>
                    {
                        app.selected_container_id().map(|container_id| {
                            if metrics_container.as_ref() != Some(&container_id) {
                                app.reset_metrics();
                                metrics_container = Some(container_id.clone());
                            }
                            last_metrics = now;
                            TaskRequest::Metrics { id: container_id }
                        })
                    }
                    5 if last_health.elapsed()
                        >= Duration::from_millis(config.metrics_interval_ms) =>
                    {
                        last_health = now;
                        app.selected_container_id()
                            .map(|id| TaskRequest::Health { id })
                    }
                    _ => None,
                };
                if let Some(request) = request {
                    task_manager.spawn_with_origin(request, TaskOrigin::Background);
                    background_cursor = (slot + 1) % 6;
                    break;
                }
            }
        }

        let timed_render =
            app.task_status == "running" && last_draw.elapsed() >= Duration::from_millis(250);
        if app.take_dirty() || timed_render {
            terminal.draw(|f| tui::draw_app(f, &mut app))?;
            last_draw = Instant::now();
        }

        if event::poll(poll_interval)?
            && let Event::Key(KeyEvent {
                code, modifiers, ..
            }) = event::read()?
        {
            app.mark_dirty();
            if pending_confirmation.is_some() {
                match code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        if let Some(request) = pending_confirmation.take() {
                            app.dismiss_latest_notification();
                            dispatch_request(
                                &mut app,
                                &mut task_manager,
                                policy,
                                &mut pending_confirmation,
                                request,
                                true,
                            );
                        }
                    }
                    KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                        pending_confirmation = None;
                        app.dismiss_latest_notification();
                        app.push_output("[action cancelled]");
                    }
                    _ => {}
                }
            } else {
                let request = if app.input_mode {
                    // Input mode: characters go to the buffer
                    match code {
                        KeyCode::Enter => app.confirm_input(),
                        KeyCode::Esc => {
                            app.cancel_input();
                            None
                        }
                        KeyCode::Backspace => {
                            app.input_buffer.pop();
                            None
                        }
                        KeyCode::Char(c) => {
                            app.input_buffer.push(c);
                            None
                        }
                        _ => None,
                    }
                } else if !matches!(app.overlay, Overlay::None) {
                    match &app.overlay {
                        Overlay::Palette { .. } => match code {
                            KeyCode::Enter => app.palette_request(),
                            KeyCode::Esc => {
                                app.overlay = Overlay::None;
                                None
                            }
                            KeyCode::Backspace => {
                                app.palette_query_backspace();
                                None
                            }
                            KeyCode::Up => {
                                app.palette_move(-1);
                                None
                            }
                            KeyCode::Down => {
                                app.palette_move(1);
                                None
                            }
                            KeyCode::Char(c) => {
                                app.palette_query_push(c);
                                None
                            }
                            _ => None,
                        },
                        Overlay::Search(_) => match code {
                            KeyCode::Enter => {
                                app.finish_search();
                                None
                            }
                            KeyCode::Esc => {
                                app.cancel_search();
                                None
                            }
                            KeyCode::Backspace => {
                                app.search_backspace();
                                None
                            }
                            KeyCode::Char(c) => {
                                app.search_push(c);
                                None
                            }
                            _ => None,
                        },
                        Overlay::Context { .. } => match code {
                            KeyCode::Enter => app.context_request_selected(),
                            KeyCode::Esc => {
                                app.overlay = Overlay::None;
                                None
                            }
                            KeyCode::Up | KeyCode::Char('k') => {
                                app.context_move(-1);
                                None
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                app.context_move(1);
                                None
                            }
                            _ => None,
                        },
                        Overlay::None => None,
                    }
                } else {
                    // Normal mode
                    match (code, modifiers) {
                        (KeyCode::Char('q'), _) => {
                            should_quit = true;
                            None
                        }
                        (KeyCode::Esc, _) => {
                            if app.is_help() {
                                app.previous_tab();
                            } else {
                                should_quit = true;
                            }
                            None
                        }
                        (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                            should_quit = true;
                            None
                        }
                        (KeyCode::Char('c'), _) => {
                            app.clear_output();
                            None
                        }
                        (KeyCode::Char('L'), _) => {
                            app.cycle_log_level();
                            None
                        }
                        (KeyCode::Char('T'), _) => {
                            app.toggle_log_timestamps();
                            None
                        }
                        (KeyCode::Backspace, _) => {
                            app.dismiss_notification();
                            None
                        }
                        (KeyCode::Char(key), _) if key == shortcuts.cancel_task => {
                            if task_manager.cancel() {
                                app.notify(NotificationKind::Info, "Task cancelled", false);
                            }
                            None
                        }
                        (KeyCode::Char(key), _) if key == shortcuts.refresh => {
                            Some(TaskRequest::ListContainers)
                        }
                        (KeyCode::Char('R'), _) => {
                            app.toggle_auto_refresh();
                            None
                        }
                        (KeyCode::Char('o'), _) => {
                            app.toggle_output();
                            None
                        }
                        (KeyCode::Char('D'), _) => {
                            app.toggle_details();
                            None
                        }
                        (KeyCode::Char('C'), _) => {
                            app.toggle_compact_layout();
                            None
                        }
                        (KeyCode::Char('d'), _) => Some(app.dashboard_request()),
                        (KeyCode::Char('i'), _) => Some(TaskRequest::ListImages),
                        (KeyCode::Char('n'), _) => Some(TaskRequest::ListNetworks),
                        (KeyCode::Char('v'), _) => Some(TaskRequest::ListVolumes),
                        (KeyCode::Char('S'), _) => Some(TaskRequest::SystemInfo),
                        (KeyCode::Char('e'), _) => Some(TaskRequest::Events),
                        (KeyCode::Char('u'), _) => Some(TaskRequest::DiskUsage { preview: false }),
                        (KeyCode::Char('K'), _) => Some(TaskRequest::DiskUsage { preview: true }),
                        (KeyCode::Char('f'), _) => {
                            app.start_filter();
                            None
                        }
                        (KeyCode::Char(':'), _) => {
                            app.start_palette();
                            None
                        }
                        (KeyCode::Char('a'), _) => {
                            app.start_context_menu();
                            None
                        }
                        (KeyCode::Char(key), _) if key == shortcuts.toggle_focus => {
                            app.toggle_focus();
                            None
                        }
                        (KeyCode::Char('s'), _) => {
                            app.toggle_container_sort();
                            None
                        }
                        (KeyCode::Char('p'), _) => {
                            app.toggle_logs_paused();
                            None
                        }
                        (KeyCode::Char('/'), _) => {
                            if !app.log_lines.is_empty() {
                                app.start_log_filter();
                            } else if matches!(
                                app.current_tab,
                                Tab::Container | Tab::Image | Tab::Network | Tab::Volume
                            ) {
                                app.start_search();
                            } else {
                                app.start_log_filter();
                            }
                            None
                        }
                        (KeyCode::Char(key), _) if key == shortcuts.theme => {
                            app.cycle_theme();
                            None
                        }
                        (KeyCode::Right, _) => {
                            app.next_tab();
                            app.current_tab_refresh_request()
                        }
                        (KeyCode::Left, _) => {
                            app.previous_tab();
                            app.current_tab_refresh_request()
                        }
                        (KeyCode::Tab, _) => {
                            app.focus_next();
                            None
                        }
                        (KeyCode::BackTab, _) => {
                            app.focus_previous();
                            None
                        }
                        (KeyCode::Up, _) => {
                            app.previous();
                            None
                        }
                        (KeyCode::Down, _) => {
                            app.next();
                            None
                        }
                        (KeyCode::Char('j'), _) => {
                            app.next();
                            None
                        }
                        (KeyCode::Char('k'), _) => {
                            app.previous();
                            None
                        }
                        (KeyCode::Char('?'), _) => {
                            app.show_help();
                            None
                        }
                        (KeyCode::PageUp, _) => {
                            app.scroll_output_up(10);
                            None
                        }
                        (KeyCode::PageDown, _) => {
                            app.scroll_output_down(10);
                            None
                        }
                        (KeyCode::Home, _) => {
                            app.scroll_output_home();
                            None
                        }
                        (KeyCode::End, _) => {
                            app.scroll_output_end();
                            None
                        }
                        (KeyCode::Enter, _) => app.execute_selected(),
                        _ => None,
                    }
                };

                if let Some(request) = request {
                    dispatch_request(
                        &mut app,
                        &mut task_manager,
                        policy,
                        &mut pending_confirmation,
                        request,
                        false,
                    );
                }
            }
        }
    }

    task_manager.cancel();
    config.theme = app.theme;
    config.auto_refresh = app.auto_refresh;
    config.show_output = app.show_output;
    config.show_details = app.show_details;
    config.compact_layout = app.compact_layout;
    config.mouse_enabled = app.mouse_enabled;
    config.save_preferences()?;
    Ok(())
}
