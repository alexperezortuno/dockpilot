use crate::tui::Palette;
use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Cell, List, ListItem, ListState, Row, Table, TableState},
};

#[allow(clippy::too_many_arguments)]
pub fn draw_table(
    f: &mut Frame,
    area: Rect,
    rows: impl Iterator<Item = Row<'static>>,
    widths: impl IntoIterator<Item = Constraint>,
    headers: impl IntoIterator<Item = Cell<'static>>,
    title: String,
    state: &mut TableState,
    colors: Palette,
) {
    let table = Table::new(rows, widths)
        .header(Row::new(headers))
        .block(Block::default().borders(Borders::ALL).title(title))
        .row_highlight_style(
            Style::default()
                .bg(colors.selection)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");
    f.render_stateful_widget(table, area, state);
}

pub fn draw_actions(
    f: &mut Frame,
    area: Rect,
    title: &str,
    items: impl Iterator<Item = ListItem<'static>>,
    state: &mut ListState,
    colors: Palette,
) {
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(
            Style::default()
                .bg(colors.selection)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");
    f.render_stateful_widget(list, area, state);
}
