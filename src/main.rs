mod app;
mod config;
mod docker;
mod security;
mod tasks;
mod tui;

use app::App;
use clap::Parser;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use docker::client::EngineConnection;
use ratatui::{Terminal, backend::CrosstermBackend};
use security::SafetyPolicy;
use std::{io, time::Duration};
use tasks::{TaskEvent, TaskManager, TaskRequest};
use tui::terminal::TerminalGuard;

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
        return;
    }
    if !confirmed && policy.requires_confirmation(mutation) {
        app.push_output(format!("[confirmation required: {} (y/n)]", description));
        *pending_confirmation = Some(request);
        return;
    }
    if !task_manager.spawn(request) {
        app.push_output("[Task in progress: press x to cancel]");
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = config::Config::load(&config::Cli::parse())?;
    let policy = SafetyPolicy::new(config.safe_mode, config.read_only);
    let _terminal_guard = TerminalGuard::new()?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::with_project_folder(config.project_folder);
    let engine_connection = EngineConnection::connect().await;
    let engine_status = engine_connection.status_message();
    let engine_status_display = engine_connection.status().to_string();
    let engine_client = engine_connection.into_client();
    app.set_engine_status(engine_status);
    app.push_output(format!("[docker] {}", engine_status_display));
    let mut task_manager = TaskManager::new(32);
    task_manager.set_client(engine_client);
    if task_manager.has_client() {
        task_manager.spawn(TaskRequest::ListContainers);
    }
    let poll_interval = Duration::from_millis(config.poll_interval_ms);
    let mut pending_confirmation: Option<TaskRequest> = None;
    let mut should_quit = false;

    while !should_quit {
        while let Some(task_event) = task_manager.try_next() {
            match task_event {
                TaskEvent::Started { id } => {
                    app.push_output(format!("[task {} started]", id));
                }
                TaskEvent::Progress { id, message } => {
                    app.push_output(format!("[task {}] {}", id, message));
                }
                TaskEvent::Finished { id, lines } => {
                    app.append_output(lines);
                    task_manager.complete(id);
                }
                TaskEvent::Containers { id, containers } => {
                    let count = containers.len();
                    app.set_containers(containers);
                    app.push_output(format!("[docker] loaded {} containers", count));
                    task_manager.complete(id);
                    if task_manager.has_client() {
                        task_manager.spawn(app.dashboard_request());
                    }
                }
                TaskEvent::LogLine { line } => {
                    app.push_log_line(line);
                }
                TaskEvent::Dashboard { id, data } => {
                    app.set_dashboard(data);
                    task_manager.complete(id);
                    if task_manager.has_client() {
                        task_manager.spawn(TaskRequest::ListImages);
                    }
                }
                TaskEvent::Images { id, images } => {
                    let count = images.len();
                    app.set_images(images);
                    app.push_output(format!("[docker] loaded {} images", count));
                    task_manager.complete(id);
                }
            }
        }

        terminal.draw(|f| tui::draw_app(f, &mut app))?;

        if event::poll(poll_interval)?
            && let Event::Key(KeyEvent {
                code, modifiers, ..
            }) = event::read()?
        {
            if pending_confirmation.is_some() {
                match code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        if let Some(request) = pending_confirmation.take() {
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
                } else {
                    // Normal mode
                    match (code, modifiers) {
                        (KeyCode::Char('q'), _) | (KeyCode::Esc, _) => {
                            should_quit = true;
                            None
                        }
                        (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                            should_quit = true;
                            None
                        }
                        (KeyCode::Char('x'), _) => {
                            if task_manager.cancel() {
                                app.push_output("[task cancelled]");
                            }
                            None
                        }
                        (KeyCode::Char('r'), _) => Some(TaskRequest::ListContainers),
                        (KeyCode::Char('d'), _) => Some(app.dashboard_request()),
                        (KeyCode::Char('i'), _) => Some(TaskRequest::ListImages),
                        (KeyCode::Char('f'), _) => {
                            app.start_container_filter();
                            None
                        }
                        (KeyCode::Char('m'), _) => {
                            app.toggle_container_focus();
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
                            app.start_log_filter();
                            None
                        }
                        (KeyCode::Tab, _) => {
                            app.next_tab();
                            None
                        }
                        (KeyCode::BackTab, _) => {
                            app.previous_tab();
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
    Ok(())
}
