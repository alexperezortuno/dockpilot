use serde::Serialize;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

impl FromStr for LogLevel {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "debug" => Ok(Self::Debug),
            "info" => Ok(Self::Info),
            "warn" | "warning" => Ok(Self::Warn),
            "error" => Ok(Self::Error),
            _ => Err(()),
        }
    }
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
