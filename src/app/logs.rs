#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

pub fn classify(line: &str) -> LogLevel {
    let lower = line.to_ascii_lowercase();
    if ["trace", "debug"]
        .iter()
        .any(|marker| lower.contains(marker))
    {
        LogLevel::Debug
    } else if ["error", "fatal", "panic", "exception"]
        .iter()
        .any(|marker| lower.contains(marker))
    {
        LogLevel::Error
    } else if ["warn", "warning"]
        .iter()
        .any(|marker| lower.contains(marker))
    {
        LogLevel::Warn
    } else {
        LogLevel::Info
    }
}

pub fn matches_level(line: &str, filter: Option<LogLevel>) -> bool {
    filter.is_none_or(|level| classify(line) >= level)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn classifies_without_changing_content() {
        let line = "2026 INFO user=ready";
        assert_eq!(classify(line), LogLevel::Info);
        assert_eq!(line, "2026 INFO user=ready");
        assert!(matches_level("warning: slow", Some(LogLevel::Warn)));
        assert!(!matches_level("debug trace", Some(LogLevel::Info)));
    }
}
