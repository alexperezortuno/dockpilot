use crossterm::{
    cursor::Show,
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use std::io::{self, Write};

pub struct TerminalGuard<W: Write> {
    writer: W,
    restore_raw_mode: bool,
}

impl TerminalGuard<io::Stdout> {
    pub fn new() -> io::Result<Self> {
        enable_raw_mode()?;
        let mut writer = io::stdout();

        if let Err(error) = execute!(writer, EnterAlternateScreen) {
            let _ = disable_raw_mode();
            return Err(error);
        }

        Ok(Self {
            writer,
            restore_raw_mode: true,
        })
    }
}

impl<W: Write> TerminalGuard<W> {
    #[cfg(test)]
    fn for_test(writer: W) -> Self {
        Self {
            writer,
            restore_raw_mode: false,
        }
    }
}

impl<W: Write> Drop for TerminalGuard<W> {
    fn drop(&mut self) {
        let _ = execute!(self.writer, LeaveAlternateScreen, Show);
        if self.restore_raw_mode {
            let _ = disable_raw_mode();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TerminalGuard;

    #[test]
    fn drop_restores_screen_and_cursor() {
        let mut output = Vec::new();
        {
            let _guard = TerminalGuard::for_test(&mut output);
        }

        let output = String::from_utf8(output).expect("terminal commands are UTF-8");
        assert!(output.contains("\u{1b}[?1049l"));
        assert!(output.contains("\u{1b}[?25h"));
    }
}
