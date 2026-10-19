use crate::{app::App, docker::exec::ExecState, tui::Palette};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Paragraph},
};
use tui_term::widget::PseudoTerminal;

fn short_id(id: &str) -> &str {
    id.get(..12).unwrap_or(id)
}

pub fn draw(f: &mut Frame, app: &mut App, area: Rect, colors: Palette) {
    let Some(exec) = app.exec.as_mut() else {
        return;
    };
    let parts = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(area);

    let title = format!(
        " Exec: {} ({}) | {} | {} ",
        exec.session.container_name,
        short_id(&exec.session.container_id),
        exec.session.command_display,
        exec.state.label()
    );
    f.render_widget(
        Paragraph::new(title).block(Block::default().borders(Borders::ALL)),
        parts[0],
    );

    let block = Block::default().borders(Borders::ALL);
    let inner = block.inner(parts[1]);
    exec.set_area(inner.height, inner.width);
    f.render_widget(block, parts[1]);
    f.render_widget(PseudoTerminal::new(exec.screen()), inner);

    let hint = match &exec.state {
        ExecState::Finished { .. } | ExecState::Failed { .. } => {
            "session closed | Esc/Enter: back to Dockpilot".to_string()
        }
        _ => "Ctrl+]: detach (process keeps running) | Shift+PgUp/PgDn: scrollback".to_string(),
    };
    f.render_widget(
        Paragraph::new(format!(" {} ", hint)).style(
            Style::default()
                .fg(colors.accent)
                .add_modifier(Modifier::BOLD),
        ),
        parts[2],
    );
}
