//! TUI-side state of an interactive exec session.
use crate::docker::exec::{ExecControl, ExecEvent, ExecSession, ExecState};

/// Scrollback kept by the terminal emulator; rendering never allocates more.
const SCROLLBACK_LIMIT: usize = 2_000;

pub struct ExecView {
    pub session: ExecSession,
    pub state: ExecState,
    parser: vt100::Parser,
    scroll_offset: usize,
    area_rows: u16,
    area_cols: u16,
}

impl ExecView {
    pub fn new(session: ExecSession, rows: u16, cols: u16) -> Self {
        let rows = rows.max(1);
        let cols = cols.max(1);
        Self {
            session,
            state: ExecState::Connecting,
            parser: vt100::Parser::new(rows, cols, SCROLLBACK_LIMIT),
            scroll_offset: 0,
            area_rows: rows,
            area_cols: cols,
        }
    }

    /// Drain pending session events; returns true when the screen changed.
    pub fn drain(&mut self) -> bool {
        let mut changed = false;
        while let Some(event) = self.session.try_next_event() {
            changed = true;
            match event {
                ExecEvent::Output(bytes) => self.parser.process(&bytes),
                ExecEvent::State(state) => self.state = state,
            }
        }
        changed
    }

    pub fn send_input(&mut self, bytes: Vec<u8>) {
        if self.state.is_closed() {
            return;
        }
        // Typing always returns the viewport to the live screen.
        self.scroll_offset = 0;
        self.parser.screen_mut().set_scrollback(0);
        self.session.send(ExecControl::Input(bytes));
    }

    /// Keep the emulator and the remote PTY aligned with the rendered area.
    pub fn set_area(&mut self, rows: u16, cols: u16) {
        let rows = rows.max(1);
        let cols = cols.max(1);
        if (rows, cols) == (self.area_rows, self.area_cols) {
            return;
        }
        self.area_rows = rows;
        self.area_cols = cols;
        self.parser.screen_mut().set_size(rows, cols);
        self.session.send(ExecControl::Resize { rows, cols });
    }

    pub fn scroll(&mut self, delta: i32) {
        let offset = self.scroll_offset.saturating_add_signed(delta as isize);
        let offset = offset.min(SCROLLBACK_LIMIT);
        // vt100 clamps to the scrollback actually available.
        self.parser.screen_mut().set_scrollback(offset);
        self.scroll_offset = self.parser.screen().scrollback();
    }

    /// Detach from the session; the remote process keeps running.
    pub fn detach(&mut self) {
        if self.state.is_closed() {
            return;
        }
        self.session.send(ExecControl::Detach);
    }

    pub fn screen(&self) -> &vt100::Screen {
        self.parser.screen()
    }
}

#[cfg(test)]
mod tests {
    use super::ExecView;
    use crate::docker::exec::{ExecCommand, ExecSession};

    fn dead_session() -> ExecSession {
        let client = bollard::Docker::connect_with_http(
            "http://127.0.0.1:9",
            1,
            bollard::API_DEFAULT_VERSION,
        )
        .expect("client with unreachable endpoint");
        ExecSession::start(
            client,
            "container-id".to_string(),
            "name".to_string(),
            ExecCommand::Sh,
            24,
            80,
        )
    }

    #[tokio::test]
    async fn failed_connection_surfaces_a_failed_state() {
        let mut view = ExecView::new(dead_session(), 24, 80);
        for _ in 0..100 {
            view.drain();
            if view.state.is_closed() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        assert!(
            matches!(view.state, crate::docker::exec::ExecState::Failed { .. }),
            "expected failed state, got {:?}",
            view.state
        );
    }

    #[tokio::test]
    async fn ansi_sequences_update_the_emulated_screen() {
        let mut view = ExecView::new(dead_session(), 5, 20);
        view.parser.process(b"$ \x1b[31mred\x1b[0m plain");
        let contents = view.screen().contents();
        assert!(contents.contains("red plain"));
        let cell = view.screen().cell(0, 2).expect("cell within screen bounds");
        assert_eq!(cell.fgcolor(), vt100::Color::Idx(1));
    }

    #[tokio::test]
    async fn fragmented_utf8_is_buffered_until_complete() {
        let mut view = ExecView::new(dead_session(), 5, 20);
        let bytes = "año 🦀".as_bytes();
        for chunk in bytes.chunks(1) {
            view.parser.process(chunk);
        }
        assert!(view.screen().contents().contains("año 🦀"));
    }

    #[tokio::test]
    async fn resize_updates_the_parser_and_queues_a_pty_resize() {
        let mut view = ExecView::new(dead_session(), 24, 80);
        view.set_area(10, 40);
        assert_eq!(view.screen().size(), (10, 40));
        // Same area is a no-op.
        view.set_area(10, 40);
        assert_eq!(view.screen().size(), (10, 40));
    }

    #[tokio::test]
    async fn scrollback_is_bounded_and_typing_snaps_back_to_live_view() {
        let mut view = ExecView::new(dead_session(), 3, 10);
        for index in 0..50 {
            view.parser
                .process(format!("line {}\r\n", index).as_bytes());
        }
        view.scroll(5);
        assert_eq!(view.scroll_offset, 5);
        view.scroll(-10_000);
        assert!(view.scroll_offset <= super::SCROLLBACK_LIMIT);
        view.send_input(b"x".to_vec());
        assert_eq!(view.scroll_offset, 0);
    }
}
