use crate::app::{App, Overlay};
use ratatui::{
    Frame,
    layout::Rect,
    widgets::{Block, Borders, Paragraph},
};

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let text = match &app.overlay {
        Overlay::Search(state) => format!("/{}", state.query),
        Overlay::UnifiedLogSearch => format!("/{}", app.unified_log_view.search_query),
        Overlay::UnifiedLogFilter => format!(
            "{}: {}",
            app.unified_log_view.filter_field.label(),
            app.unified_log_view.filter_input
        ),
        _ => return,
    };
    let count = match app.current_tab {
        crate::app::Tab::UnifiedLogs => app.unified_log_view.cached_sequences.len(),
        crate::app::Tab::Container => app.filtered_containers().len(),
        crate::app::Tab::Image => app.filtered_images().len(),
        crate::app::Tab::Network => app.filtered_networks().len(),
        crate::app::Tab::Volume => app.filtered_volumes().len(),
        _ => 0,
    };
    f.render_widget(
        Paragraph::new(format!("{}  ({} coincidencias)", text, count)).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Search/filter | Enter=apply Esc=close"),
        ),
        area,
    );
}
