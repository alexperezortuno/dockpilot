use crate::app::FocusTarget;

pub fn label(focus: FocusTarget) -> &'static str {
    match focus {
        FocusTarget::Table => "table",
        FocusTarget::Actions => "actions",
    }
}

pub fn title(resource: &str, count: usize, filter: &str, focus: FocusTarget) -> String {
    format!(
        " {resource} ({count}) | filter: {} | focus: {} ",
        if filter.is_empty() { "none" } else { filter },
        label(focus)
    )
}
