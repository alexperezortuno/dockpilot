pub fn message(resource: &str, refresh_key: char) -> String {
    format!("No {resource} available\nPress {refresh_key} to refresh")
}
