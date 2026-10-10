use crate::{
    app::{App, ContextAction, Overlay},
    tui::Palette,
};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    widgets::{Block, Borders, Clear, List, ListItem},
};

pub fn draw(f: &mut Frame, app: &App, area: Rect, colors: Palette) {
    let Overlay::Context { selected } = app.overlay else {
        return;
    };
    let actions = app.context_actions();
    if actions.is_empty() {
        return;
    }
    let width = area.width.clamp(1, 52);
    let height = (actions.len() as u16 + 2).min(area.height.max(3));
    let rect = Rect::new(
        area.x + area.width.saturating_sub(width) / 2,
        area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    );
    f.render_widget(Clear, rect);
    let items = actions.iter().enumerate().map(|(index, action)| {
        let mut item = ListItem::new(label(*action));
        if index == selected {
            item = item.style(
                Style::default()
                    .bg(colors.selection)
                    .add_modifier(Modifier::BOLD),
            );
        }
        item
    });
    f.render_widget(
        List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Context actions"),
        ),
        rect,
    );
}

fn label(action: ContextAction) -> &'static str {
    match action {
        ContextAction::Inspect => "Inspect selected resource",
        ContextAction::Start => "Start container",
        ContextAction::Stop => "Stop container",
        ContextAction::Restart => "Restart container",
        ContextAction::Logs => "Follow logs",
        ContextAction::Remove => "Remove resource (confirmation required)",
        ContextAction::History => "Image history",
    }
}
