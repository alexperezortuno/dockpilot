mod app;
mod config;
mod docker;
mod tasks;
mod tui;

use app::App;
use clap::Parser;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::{io, time::Duration};
use tasks::{TaskEvent, TaskManager};
use tui::terminal::TerminalGuard;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = config::Config::load(&config::Cli::parse())?;
    let _terminal_guard = TerminalGuard::new()?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::with_project_folder(config.project_folder);
    let mut task_manager = TaskManager::new(32);
    let poll_interval = Duration::from_millis(config.poll_interval_ms);
    let mut should_quit = false;

    while !should_quit {
        while let Some(task_event) = task_manager.try_next() {
            match task_event {
                TaskEvent::Started { id } => {
                    app.push_output(format!("[tarea {} iniciada]", id));
                }
                TaskEvent::Progress { id, message } => {
                    app.push_output(format!("[tarea {}] {}", id, message));
                }
                TaskEvent::Finished { id, lines } => {
                    app.append_output(lines);
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
                            app.push_output("[tarea cancelada]");
                        }
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

            if let Some(request) = request
                && !task_manager.spawn(request)
            {
                app.push_output("[tarea ocupada: pulse x para cancelar]");
            }
        }
    }

    task_manager.cancel();
    Ok(())
}
