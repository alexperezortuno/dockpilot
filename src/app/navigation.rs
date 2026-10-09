use ratatui::widgets::{ListState, TableState};

pub(crate) fn next_list(state: &mut ListState, len: usize) {
    if len == 0 {
        return;
    }
    let index = state.selected().map_or(0, |index| (index + 1) % len);
    state.select(Some(index));
}

pub(crate) fn previous_list(state: &mut ListState, len: usize) {
    if len == 0 {
        return;
    }
    let index = match state.selected() {
        Some(0) | None => len - 1,
        Some(index) => index - 1,
    };
    state.select(Some(index));
}

pub(crate) fn next_table(state: &mut TableState, len: usize) {
    if len == 0 {
        return;
    }
    let index = state.selected().map_or(0, |index| (index + 1) % len);
    state.select(Some(index));
}

pub(crate) fn previous_table(state: &mut TableState, len: usize) {
    if len == 0 {
        return;
    }
    let index = match state.selected() {
        Some(0) | None => len - 1,
        Some(index) => index - 1,
    };
    state.select(Some(index));
}
