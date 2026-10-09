mod app;
mod docker;
mod tui;

use app::App;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
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

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}
