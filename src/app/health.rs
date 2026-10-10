use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    Healthy,
    Unhealthy,
    Starting,
    NoHealthcheck,
    Stopped,
}

pub fn status(container_running: bool, health: Option<&str>) -> HealthStatus {
    if !container_running {
        return HealthStatus::Stopped;
    }
    match health.map(str::to_ascii_lowercase).as_deref() {
        Some("healthy") => HealthStatus::Healthy,
        Some("unhealthy") => HealthStatus::Unhealthy,
        Some("starting") => HealthStatus::Starting,
        _ => HealthStatus::NoHealthcheck,
    }
}

pub fn uptime(started_at: Option<SystemTime>, now: SystemTime) -> Option<Duration> {
    started_at.and_then(|started| now.duration_since(started).ok())
}

pub fn unix_seconds(time: SystemTime) -> Option<i64> {
    time.duration_since(UNIX_EPOCH)
        .ok()
        .map(|value| value.as_secs() as i64)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn never_infers_health_without_a_healthcheck() {
        assert_eq!(status(true, None), HealthStatus::NoHealthcheck);
        assert_eq!(status(false, Some("healthy")), HealthStatus::Stopped);
        assert_eq!(status(true, Some("unhealthy")), HealthStatus::Unhealthy);
    }
    #[test]
    fn uptime_is_safe_for_future_start_times() {
        let now = SystemTime::now();
        assert!(uptime(Some(now), now).is_some());
        assert!(uptime(Some(now + Duration::from_secs(1)), now).is_none());
    }
}
