use std::collections::VecDeque;

pub(crate) fn push(lines: &mut VecDeque<String>, capacity: usize, line: impl Into<String>) {
    lines.push_back(line.into());
    while lines.len() > capacity.max(1) {
        lines.pop_front();
    }
}

pub(crate) fn resize(lines: &mut VecDeque<String>, capacity: usize) {
    while lines.len() > capacity.max(1) {
        lines.pop_front();
    }
}
