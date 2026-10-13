use super::logs::{LogLevel, classify};
use bollard::{Docker, container::LogOutput, query_parameters::LogsOptionsBuilder};
use futures_util::StreamExt;
use serde::Serialize;
use std::{collections::VecDeque, io, path::Path};
use tokio::sync::{mpsc, watch};

pub const DEFAULT_CAPACITY: usize = 10_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum LogStream {
    Stdout,
    Stderr,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LogEvent {
    pub sequence: u64,
    pub container_id: String,
    pub container_name: String,
    pub compose_project: Option<String>,
    pub compose_service: Option<String>,
    pub timestamp: Option<String>,
    pub stream: LogStream,
    pub message: String,
    pub severity: LogLevel,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LogFilter {
    pub container: Option<String>,
    pub compose_service: Option<String>,
    pub minimum_severity: Option<LogLevel>,
    pub since: Option<String>,
    pub until: Option<String>,
    pub text: Option<String>,
}

impl LogFilter {
    fn matches(&self, event: &LogEvent) -> bool {
        let contains = |value: &str, query: &Option<String>| {
            query.as_ref().is_none_or(|query| {
                value
                    .to_ascii_lowercase()
                    .contains(&query.to_ascii_lowercase())
            })
        };
        contains(&event.container_name, &self.container)
            && contains(
                event.compose_service.as_deref().unwrap_or_default(),
                &self.compose_service,
            )
            && self
                .minimum_severity
                .is_none_or(|level| event.severity >= level)
            && self.since.as_ref().is_none_or(|since| {
                event
                    .timestamp
                    .as_ref()
                    .is_some_and(|timestamp| timestamp >= since)
            })
            && self.until.as_ref().is_none_or(|until| {
                event
                    .timestamp
                    .as_ref()
                    .is_some_and(|timestamp| timestamp <= until)
            })
            && contains(&event.message, &self.text)
    }
}

#[derive(Debug)]
pub struct LogBuffer {
    capacity: usize,
    next_sequence: u64,
    events: VecDeque<LogEvent>,
}

impl LogBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            next_sequence: 0,
            events: VecDeque::new(),
        }
    }

    pub fn push(&mut self, mut event: LogEvent) {
        event.sequence = self.next_sequence;
        self.next_sequence = self.next_sequence.wrapping_add(1);
        if self.events.len() == self.capacity {
            self.events.pop_front();
        }
        self.events.push_back(event);
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
    pub fn all(&self) -> impl Iterator<Item = &LogEvent> {
        self.events.iter()
    }

    pub fn visible(&self, filter: &LogFilter) -> Vec<&LogEvent> {
        let mut visible = self
            .events
            .iter()
            .filter(|event| filter.matches(event))
            .collect::<Vec<_>>();
        visible.sort_by(|left, right| {
            left.timestamp
                .is_none()
                .cmp(&right.timestamp.is_none())
                .then(left.timestamp.as_deref().cmp(&right.timestamp.as_deref()))
                .then(left.sequence.cmp(&right.sequence))
        });
        visible
    }
}

pub fn search_indices(events: &[&LogEvent], query: &str) -> Vec<usize> {
    let query = query.to_ascii_lowercase();
    events
        .iter()
        .enumerate()
        .filter(|(_, event)| {
            event.message.to_ascii_lowercase().contains(&query)
                || event.container_name.to_ascii_lowercase().contains(&query)
        })
        .map(|(index, _)| index)
        .collect()
}

pub fn compose_metadata(
    labels: &std::collections::HashMap<String, String>,
) -> (Option<String>, Option<String>) {
    (
        labels.get("com.docker.compose.project").cloned(),
        labels.get("com.docker.compose.service").cloned(),
    )
}

pub fn export_txt(path: &Path, events: &[&LogEvent], overwrite: bool) -> io::Result<()> {
    let mut output = String::new();
    for event in events {
        output.push_str(&format!(
            "{} [{}] {} ({:?}) {}\n",
            event.timestamp.as_deref().unwrap_or("-"),
            event.container_name,
            event.container_id,
            event.stream,
            event.message
        ));
    }
    write_export(path, output.as_bytes(), overwrite)
}

pub fn export_jsonl(path: &Path, events: &[&LogEvent], overwrite: bool) -> io::Result<()> {
    let mut output = String::new();
    for event in events {
        let line = serde_json::to_string(event)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        output.push_str(&line);
        output.push('\n');
    }
    write_export(path, output.as_bytes(), overwrite)
}

fn write_export(path: &Path, contents: &[u8], overwrite: bool) -> io::Result<()> {
    if path.as_os_str().is_empty() || path.is_dir() || (path.exists() && !overwrite) {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "invalid or existing export path",
        ));
    }
    std::fs::write(path, contents)
}

#[derive(Debug, Clone)]
pub struct LogSource {
    pub id: String,
    pub name: String,
    pub compose_project: Option<String>,
    pub compose_service: Option<String>,
}

/// Streams each container fairly through one bounded receiver. Docker ordering across
/// containers is approximate and does not establish causal ordering.
pub async fn stream_sources(
    client: Docker,
    sources: Vec<LogSource>,
    follow: bool,
    sender: mpsc::Sender<LogEvent>,
    mut cancelled: watch::Receiver<bool>,
) {
    let mut tasks = Vec::new();
    for source in sources {
        let client = client.clone();
        let sender = sender.clone();
        let mut cancelled = cancelled.clone();
        tasks.push(tokio::spawn(async move {
            let options = LogsOptionsBuilder::new()
                .follow(follow)
                .stdout(true)
                .stderr(true)
                .timestamps(true)
                .tail("200")
                .build();
            let mut stream = client.logs(&source.id, Some(options));
            while let Some(result) = tokio::select! {
                result = stream.next() => result,
                _ = cancelled.changed() => return,
            } {
                let output = match result {
                    Ok(output) => output,
                    Err(_) => return,
                };
                let (stream_kind, raw) = match output {
                    LogOutput::StdOut { message } => (LogStream::Stdout, message),
                    LogOutput::StdErr { message } => (LogStream::Stderr, message),
                    LogOutput::Console { message } => (LogStream::Stdout, message),
                    LogOutput::StdIn { message } => (LogStream::Stdout, message),
                };
                for line in String::from_utf8_lossy(&raw).lines() {
                    let (timestamp, message) = split_timestamp(line);
                    let event = LogEvent {
                        sequence: 0,
                        container_id: source.id.clone(),
                        container_name: source.name.clone(),
                        compose_project: source.compose_project.clone(),
                        compose_service: source.compose_service.clone(),
                        severity: classify(message),
                        timestamp,
                        stream: stream_kind,
                        message: message.to_string(),
                    };
                    if sender.send(event).await.is_err() {
                        return;
                    }
                }
            }
        }));
    }
    let _ = cancelled.changed().await;
    for task in tasks {
        task.abort();
    }
}

fn split_timestamp(line: &str) -> (Option<String>, &str) {
    let mut parts = line.splitn(2, ' ');
    let first = parts.next().unwrap_or_default();
    let rest = parts.next();
    let valid = first.len() >= 20
        && first.as_bytes().get(4) == Some(&b'-')
        && first.as_bytes().get(7) == Some(&b'-')
        && first.as_bytes().get(10) == Some(&b'T')
        && first.bytes().enumerate().all(|(index, byte)| {
            matches!(index, 4 | 7 | 10 | 13 | 16 | 19)
                || byte.is_ascii_digit()
                || byte == b':'
                || byte == b'.'
                || byte == b'+'
                || byte == b'-'
                || byte == b'Z'
        })
        && first.as_bytes().get(13) == Some(&b':')
        && first.as_bytes().get(16) == Some(&b':');
    if rest.is_some() && valid {
        (Some(first.to_string()), rest.unwrap_or_default())
    } else {
        (None, line)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn event(name: &str, timestamp: Option<&str>, message: &str) -> LogEvent {
        LogEvent {
            sequence: 0,
            container_id: name.into(),
            container_name: name.into(),
            compose_project: None,
            compose_service: None,
            timestamp: timestamp.map(str::to_string),
            stream: LogStream::Stdout,
            message: message.into(),
            severity: classify(message),
        }
    }

    #[test]
    fn buffer_is_bounded_and_orders_timestamp_with_stable_ties() {
        let mut buffer = LogBuffer::new(2);
        buffer.push(event("a", Some("2026-01-01T00:00:02Z"), "second"));
        buffer.push(event("b", Some("2026-01-01T00:00:01Z"), "first"));
        buffer.push(event("c", None, "latest"));
        let visible = buffer.visible(&LogFilter::default());
        assert_eq!(buffer.len(), 2);
        assert_eq!(visible[0].message, "first");
        assert_eq!(visible[1].message, "latest");
    }

    #[test]
    fn combined_filter_and_incremental_search_are_case_insensitive() {
        let mut buffer = LogBuffer::new(4);
        let mut api = event("api", Some("2026-01-01T00:00:01Z"), "ERROR failed");
        api.compose_service = Some("web".into());
        buffer.push(api);
        buffer.push(event("db", Some("2026-01-01T00:00:02Z"), "ready"));
        let filter = LogFilter {
            container: Some("API".into()),
            compose_service: Some("WEB".into()),
            minimum_severity: Some(LogLevel::Error),
            text: Some("FAIL".into()),
            ..Default::default()
        };
        let visible = buffer.visible(&filter);
        assert_eq!(search_indices(&visible, "failed"), vec![0]);
        assert_eq!(
            compose_metadata(&HashMap::from([(
                String::from("com.docker.compose.service"),
                String::from("web")
            )]))
            .1
            .as_deref(),
            Some("web")
        );
    }

    #[test]
    fn absent_or_invalid_timestamps_do_not_panic() {
        assert_eq!(
            split_timestamp("not-a-timestamp message"),
            (None, "not-a-timestamp message")
        );
        assert_eq!(
            split_timestamp("2026-01-01T00:00:00Z message"),
            (Some("2026-01-01T00:00:00Z".into()), "message")
        );
        assert_eq!(split_timestamp("2026-xx-01T00:00:00Z message").0, None);
    }

    #[test]
    fn exports_jsonl_and_rejects_overwrite_without_confirmation() {
        let path =
            std::env::temp_dir().join(format!("dockpilot-unified-{}.jsonl", std::process::id()));
        let buffer = LogBuffer::new(2);
        let events = buffer.visible(&LogFilter::default());
        let _ = std::fs::remove_file(&path);
        export_jsonl(&path, &events, false).unwrap();
        assert!(export_jsonl(&path, &events, false).is_err());
        export_jsonl(&path, &events, true).unwrap();
        std::fs::remove_file(path).unwrap();
    }
}
