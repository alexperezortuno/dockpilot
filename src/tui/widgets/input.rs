use crate::app::App;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
};

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let text = format!("{} {}", app.input_prompt, app.input_buffer);
    f.render_widget(
        Paragraph::new(text).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Entry (Enter=OK, Esc=Cancel) ")
                .border_style(Style::default().fg(Color::Yellow)),
        ),
        area,
    );
}
