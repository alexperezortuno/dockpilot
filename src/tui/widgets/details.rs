use ratatui::{
    Frame,
    layout::Rect,
    widgets::{Block, Borders, Paragraph},
};

pub fn draw(f: &mut Frame, area: Rect, title: impl Into<String>, text: impl Into<String>) {
    f.render_widget(
        Paragraph::new(text.into())
            .block(Block::default().borders(Borders::ALL).title(title.into())),
        area,
    );
}
