use crate::app::{App, Overlay};
use ratatui::{
    Frame,
    layout::Rect,
    widgets::{Block, Borders, Paragraph},
};

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let Overlay::Search(state) = &app.overlay else {
        return;
    };
    let count = match app.current_tab {
        crate::app::Tab::Container => app.filtered_containers().len(),
        crate::app::Tab::Image => app.filtered_images().len(),
        crate::app::Tab::Network => app.filtered_networks().len(),
        crate::app::Tab::Volume => app.filtered_volumes().len(),
        _ => 0,
    };
    f.render_widget(
        Paragraph::new(format!("/{}  ({} coincidencias)", state.query, count)).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Search | Enter=apply Esc=restore"),
        ),
        area,
    );
}
