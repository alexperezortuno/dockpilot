use ratatui::layout::{Constraint, Direction, Layout, Rect};

#[derive(Debug, Clone, Copy)]
pub struct LayoutAreas {
    pub header: Rect,
    pub content: Rect,
    pub output: Rect,
    pub footer: Rect,
    pub input: Option<Rect>,
}

pub fn areas(area: Rect, input_visible: bool, show_output: bool, compact: bool) -> LayoutAreas {
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
        .constraints(if compact {
            vec![
                Constraint::Length(2),
                Constraint::Min(1),
                Constraint::Length(1),
            ]
        } else {
            vec![
                Constraint::Length(3),
                Constraint::Min(1),
                Constraint::Length(2),
            ]
        })
        .split(outer.0);
    let body = if show_output {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(5), Constraint::Min(5)])
            .split(parts[1])
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(0)])
            .split(parts[1])
    };
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
        let layout = areas(area, false, true, false);
        assert!(layout.header.bottom() <= area.bottom());
        assert!(layout.content.bottom() <= area.bottom());
        assert!(layout.footer.bottom() <= area.bottom());
    }

    #[test]
    fn hidden_output_gives_content_the_available_height() {
        let area = Rect::new(0, 0, 80, 24);
        let visible = areas(area, false, true, false);
        let hidden = areas(area, false, false, false);
        assert!(hidden.content.height > visible.content.height);
        assert_eq!(hidden.output.height, 0);
    }
}
