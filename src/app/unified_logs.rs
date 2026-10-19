use super::logs::{LogLevel, classify};
use bollard::{Docker, container::LogOutput, query_parameters::LogsOptionsBuilder};
use futures_util::StreamExt;
use serde::Serialize;
use std::{collections::VecDeque, io, path::Path};
use tokio::{
    sync::{mpsc, watch},
    task::JoinSet,
};

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
    pub compose_project: Option<String>,
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
                event.compose_project.as_deref().unwrap_or_default(),
                &self.compose_project,
            )
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
    generation: u64,
    dropped: u64,
}

impl LogBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            next_sequence: 0,
            events: VecDeque::new(),
            generation: 0,
            dropped: 0,
        }
    }

    pub fn push(&mut self, mut event: LogEvent) {
        event.sequence = self.next_sequence;
        self.next_sequence = self.next_sequence.wrapping_add(1);
        if self.events.len() == self.capacity {
            self.events.pop_front();
            self.dropped = self.dropped.saturating_add(1);
        }
        self.events.push_back(event);
        self.generation = self.generation.wrapping_add(1);
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

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn dropped(&self) -> u64 {
        self.dropped
    }

    pub fn get(&self, sequence: u64) -> Option<&LogEvent> {
        self.events.iter().find(|event| event.sequence == sequence)
    }

    pub fn visible_sequences(&self, filter: &LogFilter) -> Vec<u64> {
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
        visible.into_iter().map(|event| event.sequence).collect()
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

#[derive(Debug)]
pub struct LineAssembler {
    pending: Vec<u8>,
    max_line_bytes: usize,
}

impl LineAssembler {
    pub fn new(max_line_bytes: usize) -> Self {
        Self {
            pending: Vec::new(),
            max_line_bytes: max_line_bytes.max(1),
        }
    }

    pub fn push(&mut self, bytes: &[u8]) -> Vec<String> {
        self.pending.extend_from_slice(bytes);
        let mut lines = Vec::new();
        while let Some(position) = self.pending.iter().position(|byte| *byte == b'\n') {
            let line = self.pending.drain(..=position).collect::<Vec<_>>();
            lines.push(decode_line(
                &line[..line.len().saturating_sub(1)],
                self.max_line_bytes,
            ));
        }
        if self.pending.len() > self.max_line_bytes {
            let overflow = self.pending.split_off(self.max_line_bytes);
            let line = std::mem::replace(&mut self.pending, overflow);
            lines.push(decode_line(&line, self.max_line_bytes));
        }
        lines
    }

    pub fn finish(&mut self) -> Option<String> {
        (!self.pending.is_empty()).then(|| {
            let line = std::mem::take(&mut self.pending);
            decode_line(&line, self.max_line_bytes)
        })
    }
}

fn decode_line(bytes: &[u8], max_line_bytes: usize) -> String {
    String::from_utf8_lossy(&bytes[..bytes.len().min(max_line_bytes)])
        .trim_end_matches('\r')
        .to_string()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterField {
    Container,
    ComposeProject,
    ComposeService,
    Severity,
    Since,
    Until,
    Text,
}

impl FilterField {
    pub fn label(self) -> &'static str {
        match self {
            Self::Container => "container",
            Self::ComposeProject => "project",
            Self::ComposeService => "service",
            Self::Severity => "severity",
            Self::Since => "since",
            Self::Until => "until",
            Self::Text => "text",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Container => Self::ComposeProject,
            Self::ComposeProject => Self::ComposeService,
            Self::ComposeService => Self::Severity,
            Self::Severity => Self::Since,
            Self::Since => Self::Until,
            Self::Until => Self::Text,
            Self::Text => Self::Container,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LogViewState {
    pub filter: LogFilter,
    pub filter_field: FilterField,
    pub filter_input: String,
    pub filter_editing: bool,
    pub search_query: String,
    pub search_hits: Vec<u64>,
    pub search_cursor: usize,
    pub search_active: bool,
    pub follow: bool,
    pub paused: bool,
    pub scroll: usize,
    pub selected_sequence: Option<u64>,
    pub cached_sequences: Vec<u64>,
    cached_generation: u64,
}

impl Default for LogViewState {
    fn default() -> Self {
        Self {
            filter: LogFilter::default(),
            filter_field: FilterField::Container,
            filter_input: String::new(),
            filter_editing: false,
            search_query: String::new(),
            search_hits: Vec::new(),
            search_cursor: 0,
            search_active: false,
            follow: true,
            paused: false,
            scroll: 0,
            selected_sequence: None,
            cached_sequences: Vec::new(),
            cached_generation: u64::MAX,
        }
    }
}

impl LogViewState {
    pub fn refresh_cache(&mut self, buffer: &LogBuffer) {
        if self.cached_generation == buffer.generation() {
            return;
        }
        self.cached_sequences = buffer.visible_sequences(&self.filter);
        self.cached_generation = buffer.generation();
        self.search_hits =
            search_indices_by_sequence(buffer, &self.cached_sequences, &self.search_query);
        self.search_cursor = self
            .search_cursor
            .min(self.search_hits.len().saturating_sub(1));
        self.selected_sequence = self
            .selected_sequence
            .filter(|sequence| self.cached_sequences.contains(sequence))
            .or_else(|| self.cached_sequences.last().copied());
        self.scroll = self
            .scroll
            .min(self.cached_sequences.len().saturating_sub(1));
    }

    pub fn set_filter(&mut self, filter: LogFilter) {
        self.filter = filter;
        self.cached_generation = u64::MAX;
    }

    pub fn set_search(&mut self, query: String, buffer: &LogBuffer) {
        self.search_query = query;
        self.search_active = true;
        self.refresh_cache(buffer);
        self.search_hits =
            search_indices_by_sequence(buffer, &self.cached_sequences, &self.search_query);
        self.search_cursor = 0;
        if let Some(sequence) = self.search_hits.first() {
            self.selected_sequence = Some(*sequence);
        }
    }
}

pub fn search_indices_by_sequence(buffer: &LogBuffer, sequences: &[u64], query: &str) -> Vec<u64> {
    let query = query.to_ascii_lowercase();
    sequences
        .iter()
        .copied()
        .filter(|sequence| {
            buffer.get(*sequence).is_some_and(|event| {
                event.message.to_ascii_lowercase().contains(&query)
                    || event.container_name.to_ascii_lowercase().contains(&query)
                    || event
                        .compose_service
                        .as_deref()
                        .unwrap_or_default()
                        .to_ascii_lowercase()
                        .contains(&query)
            })
        })
        .collect()
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
    let mut tasks = JoinSet::new();
    for source in sources {
        let client = client.clone();
        let sender = sender.clone();
        let mut cancelled = cancelled.clone();
        tasks.spawn(async move {
            let mut assemblers = [LineAssembler::new(64 * 1024), LineAssembler::new(64 * 1024)];
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
                    Err(error) => {
                        let _ = sender
                            .send(LogEvent {
                                sequence: 0,
                                container_id: source.id.clone(),
                                container_name: source.name.clone(),
                                compose_project: source.compose_project.clone(),
                                compose_service: source.compose_service.clone(),
                                timestamp: None,
                                stream: LogStream::Stderr,
                                message: format!("[stream error] {error}"),
                                severity: LogLevel::Error,
                            })
                            .await;
                        return;
                    }
                };
                let (stream_kind, raw) = match output {
                    LogOutput::StdOut { message } => (LogStream::Stdout, message),
                    LogOutput::StdErr { message } => (LogStream::Stderr, message),
                    LogOutput::Console { message } => (LogStream::Stdout, message),
                    LogOutput::StdIn { message } => (LogStream::Stdout, message),
                };
                let stream_index = usize::from(stream_kind == LogStream::Stderr);
                for line in assemblers[stream_index].push(&raw) {
                    let (timestamp, message) = split_timestamp(&line);
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
            for (index, assembler) in assemblers.iter_mut().enumerate() {
                if let Some(line) = assembler.finish() {
                    let (timestamp, message) = split_timestamp(&line);
                    let _ = sender
                        .send(LogEvent {
                            sequence: 0,
                            container_id: source.id.clone(),
                            container_name: source.name.clone(),
                            compose_project: source.compose_project.clone(),
                            compose_service: source.compose_service.clone(),
                            timestamp,
                            stream: if index == 0 {
                                LogStream::Stdout
                            } else {
                                LogStream::Stderr
                            },
                            severity: classify(message),
                            message: message.to_string(),
                        })
                        .await;
                }
            }
        });
    }
    loop {
        tokio::select! {
            _ = cancelled.changed() => {
                tasks.abort_all();
                while tasks.join_next().await.is_some() {}
                break;
            }
            result = tasks.join_next() => {
                if result.is_none() { break; }
            }
        }
    }
}

pub fn split_timestamp(line: &str) -> (Option<String>, &str) {
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

    #[test]
    fn assembler_reconstructs_fragmented_lines_and_utf8() {
        let mut assembler = LineAssembler::new(32);
        assert!(assembler.push("hé".as_bytes()).is_empty());
        assert_eq!(assembler.push("llo\nnext".as_bytes()), vec!["héllo"]);
        assert_eq!(assembler.finish().as_deref(), Some("next"));
    }

    #[test]
    fn assembler_bounds_a_line_without_dropping_the_following_fragment() {
        let mut assembler = LineAssembler::new(4);
        assert_eq!(assembler.push(b"12345678"), vec!["1234"]);
        assert_eq!(assembler.push(b"\n"), vec!["5678"]);
    }

    #[test]
    fn view_cache_handles_the_full_buffer_without_rendering_every_event() {
        let mut buffer = LogBuffer::new(DEFAULT_CAPACITY);
        for index in 0..DEFAULT_CAPACITY {
            buffer.push(event("api", None, &format!("line {index}")));
        }
        let mut view = LogViewState::default();
        view.refresh_cache(&buffer);
        assert_eq!(view.cached_sequences.len(), DEFAULT_CAPACITY);
        assert_eq!(view.cached_sequences.last(), Some(&9_999));
        view.selected_sequence = Some(9_990);
        view.set_filter(LogFilter {
            text: Some("line 99".into()),
            ..Default::default()
        });
        view.refresh_cache(&buffer);
        assert!(view.cached_sequences.len() < DEFAULT_CAPACITY);
        assert!(
            view.cached_sequences
                .iter()
                .all(|sequence| { buffer.get(*sequence).unwrap().message.contains("line 99") })
        );
    }
}
