mod app;
mod docker;
mod tui;

use app::App;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io;
use tui::terminal::TerminalGuard;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _terminal_guard = TerminalGuard::new()?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    let mut should_quit = false;

    while !should_quit {
        terminal.draw(|f| tui::draw_app(f, &mut app))?;

        if let Event::Key(KeyEvent {
            code, modifiers, ..
        }) = event::read()?
        {
            if app.input_mode {
                // Input mode: characters go to the buffer
                match code {
                    KeyCode::Enter => app.confirm_input(),
                    KeyCode::Esc => app.cancel_input(),
                    KeyCode::Backspace => {
                        app.input_buffer.pop();
                    }
                    KeyCode::Char(c) => {
                        app.input_buffer.push(c);
                    }
                    _ => {}
                }
            } else {
                // Normal mode
                match (code, modifiers) {
                    (KeyCode::Char('q'), _) | (KeyCode::Esc, _) => should_quit = true,
                    (KeyCode::Char('c'), KeyModifiers::CONTROL) => should_quit = true,
                    (KeyCode::Tab, _) => app.next_tab(),
                    (KeyCode::BackTab, _) => app.previous_tab(),
                    (KeyCode::Up, _) => app.previous(),
                    (KeyCode::Down, _) => app.next(),
                    (KeyCode::Enter, _) => app.execute_selected(),
                    _ => {}
                }
            }
        }
    }

    Ok(())
}
