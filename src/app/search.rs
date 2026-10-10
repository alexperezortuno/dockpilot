#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchState {
    pub previous_filter: String,
    pub query: String,
}

impl SearchState {
    pub fn new(previous_filter: impl Into<String>) -> Self {
        let previous_filter = previous_filter.into();
        Self {
            query: previous_filter.clone(),
            previous_filter,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SearchState;

    #[test]
    fn search_starts_with_the_current_filter_and_keeps_its_snapshot() {
        let state = SearchState::new("api");
        assert_eq!(state.query, "api");
        assert_eq!(state.previous_filter, "api");
    }
}
