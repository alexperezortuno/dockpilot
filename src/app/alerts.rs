use std::collections::HashMap;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AlertKind {
    Cpu,
    Memory,
    Unhealthy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertSeverity {
    Warning,
    Critical,
}

#[derive(Debug, Clone, Copy)]
pub struct AlertConfig {
    pub cpu_percent: Option<f64>,
    pub memory_percent: Option<f64>,
    pub cooldown: Duration,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Alert {
    pub kind: AlertKind,
    pub severity: AlertSeverity,
    pub message: String,
}

#[derive(Debug, Default)]
pub struct AlertTracker {
    last: HashMap<(String, AlertKind), Instant>,
}

impl AlertTracker {
    pub fn check(
        &mut self,
        id: &str,
        cpu: Option<f64>,
        memory: Option<f64>,
        unhealthy: bool,
        config: AlertConfig,
        now: Instant,
    ) -> Vec<Alert> {
        let mut alerts = Vec::new();
        if config
            .cpu_percent
            .is_some_and(|threshold| cpu.is_some_and(|value| value >= threshold))
        {
            alerts.push(Alert {
                kind: AlertKind::Cpu,
                severity: AlertSeverity::Warning,
                message: format!("{id}: CPU above threshold"),
            });
        }
        if config
            .memory_percent
            .is_some_and(|threshold| memory.is_some_and(|value| value >= threshold))
        {
            alerts.push(Alert {
                kind: AlertKind::Memory,
                severity: AlertSeverity::Warning,
                message: format!("{id}: memory above threshold"),
            });
        }
        if unhealthy {
            alerts.push(Alert {
                kind: AlertKind::Unhealthy,
                severity: AlertSeverity::Critical,
                message: format!("{id}: container unhealthy"),
            });
        }
        alerts
            .into_iter()
            .filter(|alert| {
                let key = (id.to_string(), alert.kind);
                if self
                    .last
                    .get(&key)
                    .is_some_and(|last| now.duration_since(*last) < config.cooldown)
                {
                    return false;
                }
                self.last.insert(key, now);
                true
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cooldown_suppresses_and_allows_recovery_alert() {
        let mut tracker = AlertTracker::default();
        let config = AlertConfig {
            cpu_percent: Some(80.0),
            memory_percent: None,
            cooldown: Duration::from_secs(10),
        };
        let now = Instant::now();
        assert_eq!(
            tracker
                .check("c", Some(90.0), None, false, config, now)
                .len(),
            1
        );
        assert!(
            tracker
                .check(
                    "c",
                    Some(90.0),
                    None,
                    false,
                    config,
                    now + Duration::from_secs(1)
                )
                .is_empty()
        );
        assert_eq!(
            tracker
                .check(
                    "c",
                    Some(90.0),
                    None,
                    false,
                    config,
                    now + Duration::from_secs(11)
                )
                .len(),
            1
        );
    }
}
