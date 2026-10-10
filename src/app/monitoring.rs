use std::collections::VecDeque;
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RawSample {
    pub cpu_total: u64,
    pub system_total: u64,
    pub online_cpus: u64,
    pub memory_usage: u64,
    pub memory_limit: u64,
    pub network_rx: u64,
    pub network_tx: u64,
    pub block_read: u64,
    pub block_write: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MetricSample {
    pub at: Instant,
    pub cpu_percent: Option<f64>,
    pub memory_bytes: Option<u64>,
    pub memory_percent: Option<f64>,
    pub network_rx_per_second: Option<f64>,
    pub network_tx_per_second: Option<f64>,
    pub block_read_per_second: Option<f64>,
    pub block_write_per_second: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct MetricsHistory {
    capacity: usize,
    samples: VecDeque<MetricSample>,
    previous: Option<(Instant, RawSample)>,
}

impl MetricsHistory {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            samples: VecDeque::new(),
            previous: None,
        }
    }

    pub fn push(&mut self, at: Instant, current: RawSample) -> MetricSample {
        let sample = self
            .previous
            .and_then(|(previous_at, previous)| delta(previous_at, previous, at, current))
            .unwrap_or(MetricSample {
                at,
                cpu_percent: None,
                memory_bytes: Some(current.memory_usage),
                memory_percent: percent(current.memory_usage, current.memory_limit),
                network_rx_per_second: None,
                network_tx_per_second: None,
                block_read_per_second: None,
                block_write_per_second: None,
            });
        self.previous = Some((at, current));
        self.samples.push_back(sample);
        while self.samples.len() > self.capacity {
            self.samples.pop_front();
        }
        sample
    }

    pub fn samples(&self) -> impl Iterator<Item = &MetricSample> {
        self.samples.iter()
    }
    pub fn len(&self) -> usize {
        self.samples.len()
    }
}

fn delta(
    previous_at: Instant,
    previous: RawSample,
    at: Instant,
    current: RawSample,
) -> Option<MetricSample> {
    let seconds = at.checked_duration_since(previous_at)?.as_secs_f64();
    if seconds <= 0.0
        || current.cpu_total < previous.cpu_total
        || current.system_total < previous.system_total
    {
        return None;
    }
    let system_delta = current.system_total - previous.system_total;
    let cpu_percent = (system_delta > 0).then(|| {
        (current.cpu_total.saturating_sub(previous.cpu_total) as f64 / system_delta as f64)
            * current.online_cpus.max(1) as f64
            * 100.0
    });
    Some(MetricSample {
        at,
        cpu_percent,
        memory_bytes: Some(current.memory_usage),
        memory_percent: percent(current.memory_usage, current.memory_limit),
        network_rx_per_second: rate(previous.network_rx, current.network_rx, seconds),
        network_tx_per_second: rate(previous.network_tx, current.network_tx, seconds),
        block_read_per_second: rate(previous.block_read, current.block_read, seconds),
        block_write_per_second: rate(previous.block_write, current.block_write, seconds),
    })
}

fn rate(previous: u64, current: u64, seconds: f64) -> Option<f64> {
    (current >= previous && seconds > 0.0).then(|| (current - previous) as f64 / seconds)
}

fn percent(value: u64, limit: u64) -> Option<f64> {
    (limit > 0).then(|| value as f64 / limit as f64 * 100.0)
}

pub fn format_rate(value: Option<f64>) -> String {
    value.map(format_bytes).unwrap_or_else(|| "N/A".to_string())
}

fn format_bytes(value: f64) -> String {
    const UNITS: [&str; 4] = ["B/s", "KiB/s", "MiB/s", "GiB/s"];
    let mut value = value;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    format!("{value:.1} {}", UNITS[unit])
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn calculates_elapsed_rates_and_cpu() {
        let start = Instant::now();
        let mut history = MetricsHistory::new(3);
        history.push(
            start,
            RawSample {
                cpu_total: 10,
                system_total: 100,
                online_cpus: 2,
                memory_usage: 50,
                memory_limit: 100,
                network_rx: 10,
                network_tx: 20,
                block_read: 30,
                block_write: 40,
            },
        );
        let sample = history.push(
            start + std::time::Duration::from_secs(2),
            RawSample {
                cpu_total: 30,
                system_total: 200,
                online_cpus: 2,
                memory_usage: 75,
                memory_limit: 100,
                network_rx: 110,
                network_tx: 220,
                block_read: 230,
                block_write: 240,
            },
        );
        assert_eq!(sample.cpu_percent, Some(40.0));
        assert_eq!(sample.network_rx_per_second, Some(50.0));
        assert_eq!(sample.memory_percent, Some(75.0));
    }
    #[test]
    fn handles_reset_zero_and_bounded_history() {
        let start = Instant::now();
        let raw = RawSample {
            cpu_total: 10,
            system_total: 10,
            online_cpus: 0,
            memory_usage: 1,
            memory_limit: 0,
            network_rx: 2,
            network_tx: 2,
            block_read: 2,
            block_write: 2,
        };
        let mut history = MetricsHistory::new(2);
        history.push(start, raw);
        let invalid = history.push(
            start + std::time::Duration::from_secs(1),
            RawSample {
                cpu_total: 1,
                ..raw
            },
        );
        assert_eq!(invalid.cpu_percent, None);
        history.push(start + std::time::Duration::from_secs(2), raw);
        history.push(start + std::time::Duration::from_secs(3), raw);
        assert_eq!(history.len(), 2);
        assert_eq!(percent(1, 0), None);
    }
}
