use ratatui::layout::{Constraint, Direction, Layout, Rect};

#[derive(Debug, Clone, Copy)]
pub struct LayoutAreas {
    pub header: Rect,
    pub content: Rect,
    pub output: Rect,
    pub footer: Rect,
    pub input: Option<Rect>,
}

pub fn areas(area: Rect, input_visible: bool) -> LayoutAreas {
    let outer = if input_visible {
        let parts = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(5), Constraint::Length(3)])
            .split(area);
        (parts[0], Some(parts[1]))
    } else {
        (area, None)
    };
    let parts = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(2),
        ])
        .split(outer.0);
    let body = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Min(5)])
        .split(parts[1]);
    LayoutAreas {
        header: parts[0],
        content: body[0],
        output: body[1],
        footer: parts[2],
        input: outer.1,
    }
}

#[cfg(test)]
mod tests {
    use super::areas;
    use ratatui::layout::Rect;

    #[test]
    fn small_terminal_layout_stays_within_bounds() {
        let area = Rect::new(0, 0, 60, 20);
        let layout = areas(area, false);
        assert!(layout.header.bottom() <= area.bottom());
        assert!(layout.content.bottom() <= area.bottom());
        assert!(layout.footer.bottom() <= area.bottom());
    }
}
