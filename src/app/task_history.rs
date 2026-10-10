use std::collections::VecDeque;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct TaskRecord {
    pub id: u64,
    pub description: String,
    pub state: TaskState,
    pub started: Option<Instant>,
    pub finished: Option<Instant>,
    pub progress: Option<u8>,
}

impl TaskRecord {
    pub fn duration(&self, now: Instant) -> Duration {
        match (self.started, self.finished) {
            (Some(started), Some(finished)) => finished.saturating_duration_since(started),
            (Some(started), None) => now.saturating_duration_since(started),
            _ => Duration::ZERO,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TaskHistory {
    max: usize,
    records: VecDeque<TaskRecord>,
}

impl TaskHistory {
    pub fn new(max: usize) -> Self {
        Self {
            max: max.max(1),
            records: VecDeque::new(),
        }
    }
    pub fn queue(&mut self, id: u64, description: String) {
        self.records.push_back(TaskRecord {
            id,
            description,
            state: TaskState::Queued,
            started: None,
            finished: None,
            progress: None,
        });
        self.trim();
    }
    pub fn start(&mut self, id: u64, at: Instant) {
        if let Some(record) = self.find(id) {
            record.state = TaskState::Running;
            record.started = Some(at);
        }
    }
    pub fn finish(&mut self, id: u64, state: TaskState, at: Instant) {
        if let Some(record) = self.find(id) {
            record.state = state;
            record.finished = Some(at);
        }
    }
    pub fn set_progress(&mut self, id: u64, progress: Option<u8>) {
        if let Some(record) = self.find(id) {
            record.progress = progress;
        }
    }
    pub fn cancel(&mut self, id: u64, at: Instant) {
        self.finish(id, TaskState::Cancelled, at);
    }
    pub fn records(&self) -> impl DoubleEndedIterator<Item = &TaskRecord> {
        self.records.iter()
    }

    pub fn recent_summary(&self, now: Instant) -> Vec<String> {
        self.records
            .iter()
            .rev()
            .take(3)
            .map(|record| {
                format!(
                    "#{} {} {:?} {}s",
                    record.id,
                    record.description,
                    record.state,
                    record.duration(now).as_secs()
                )
            })
            .collect()
    }
    fn find(&mut self, id: u64) -> Option<&mut TaskRecord> {
        self.records.iter_mut().find(|record| record.id == id)
    }
    fn trim(&mut self) {
        while self.records.len() > self.max {
            self.records.pop_front();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn states_are_typed_and_bounded() {
        let mut history = TaskHistory::new(1);
        history.queue(1, "one".into());
        history.queue(2, "two".into());
        history.start(2, Instant::now());
        history.cancel(2, Instant::now());
        assert_eq!(
            history.records().next().unwrap().state,
            TaskState::Cancelled
        );
    }

    #[test]
    fn completed_duration_is_finished_minus_started() {
        let started = Instant::now();
        let finished = started + Duration::from_secs(3);
        let mut history = TaskHistory::new(2);
        history.queue(1, "one".into());
        history.start(1, started);
        history.finish(1, TaskState::Completed, finished);

        assert_eq!(
            history.records().next().unwrap().duration(finished),
            Duration::from_secs(3)
        );
    }
}
