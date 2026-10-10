use ratatui::{
    style::{Color, Style},
    text::Line,
};

pub fn footer_line(focus: &str, task: &str) -> Line<'static> {
    Line::from(format!(
        " Focus: {} | Task: {} | Tab: focus  Left/Right: tabs  ?: help ",
        focus, task
    ))
    .style(Style::default().fg(Color::DarkGray))
}
