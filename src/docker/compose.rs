use std::{
    collections::BTreeMap,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComposeContainer {
    pub id: String,
    pub service: String,
    pub name: String,
    pub state: String,
    pub status: String,
    pub image: String,
    pub ports: String,
    pub health: String,
    pub restarts: i64,
    pub started_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComposeService {
    pub name: String,
    pub containers: Vec<ComposeContainer>,
    pub depends_on: Vec<String>,
}

impl ComposeService {
    pub fn status(&self) -> String {
        if self.containers.is_empty() {
            return "not running".to_string();
        }
        if self
            .containers
            .iter()
            .all(|container| container.state.eq_ignore_ascii_case("running"))
        {
            "running".to_string()
        } else if self
            .containers
            .iter()
            .any(|container| container.state.eq_ignore_ascii_case("running"))
        {
            "partial".to_string()
        } else {
            "stopped".to_string()
        }
    }

    pub fn health(&self) -> String {
        if self.containers.is_empty() {
            return "unavailable".to_string();
        }
        if self
            .containers
            .iter()
            .any(|container| container.health.eq_ignore_ascii_case("unhealthy"))
        {
            return "unhealthy".to_string();
        }
        if self
            .containers
            .iter()
            .any(|container| container.health.eq_ignore_ascii_case("starting"))
        {
            return "starting".to_string();
        }
        if self
            .containers
            .iter()
            .all(|container| container.health.eq_ignore_ascii_case("no-check"))
        {
            "no-check".to_string()
        } else {
            "healthy".to_string()
        }
    }

    pub fn replicas(&self) -> String {
        format!("{}", self.containers.len())
    }

    pub fn images(&self) -> String {
        unique_values(
            self.containers
                .iter()
                .map(|container| container.image.as_str()),
        )
    }

    pub fn ports(&self) -> String {
        unique_values(
            self.containers
                .iter()
                .map(|container| container.ports.as_str())
                .filter(|ports| !ports.is_empty()),
        )
    }

    pub fn restarts(&self) -> i64 {
        self.containers
            .iter()
            .map(|container| container.restarts)
            .sum()
    }

    pub fn uptime(&self, now: SystemTime) -> String {
        self.containers
            .iter()
            .filter_map(|container| container.started_at.as_deref())
            .filter_map(parse_docker_timestamp)
            .filter_map(|started| now.duration_since(started).ok())
            .min()
            .map(format_duration)
            .unwrap_or_else(|| "-".to_string())
    }
}

fn unique_values<'a>(values: impl Iterator<Item = &'a str>) -> String {
    let mut unique = Vec::new();
    for value in values {
        if !unique.iter().any(|current| current == &value) {
            unique.push(value);
        }
    }
    unique.join(", ")
}

fn format_duration(duration: Duration) -> String {
    let seconds = duration.as_secs();
    if seconds < 60 {
        format!("{seconds}s")
    } else if seconds < 3_600 {
        format!("{}m", seconds / 60)
    } else if seconds < 86_400 {
        format!("{}h {}m", seconds / 3_600, (seconds / 60) % 60)
    } else {
        format!("{}d {}h", seconds / 86_400, (seconds / 3_600) % 24)
    }
}

fn parse_docker_timestamp(value: &str) -> Option<SystemTime> {
    let (date, time) = value.trim_end_matches('Z').split_once('T')?;
    let mut date = date.split('-');
    let year: i64 = date.next()?.parse().ok()?;
    let month: i64 = date.next()?.parse().ok()?;
    let day: i64 = date.next()?.parse().ok()?;
    let time = time.split('.').next()?;
    let mut time = time.split(':');
    let hour: i64 = time.next()?.parse().ok()?;
    let minute: i64 = time.next()?.parse().ok()?;
    let second: i64 = time.next()?.parse().ok()?;
    let adjusted_year = year - i64::from(month <= 2);
    let era = if adjusted_year >= 0 {
        adjusted_year / 400
    } else {
        (adjusted_year - 399) / 400
    };
    let year_of_era = adjusted_year - era * 400;
    let month_of_year = month + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * month_of_year + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    let days = era * 146_097 + day_of_era - 719_468;
    let seconds = days * 86_400 + hour * 3_600 + minute * 60 + second;
    UNIX_EPOCH.checked_add(Duration::from_secs(seconds.try_into().ok()?))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComposeProject {
    pub context: String,
    pub name: String,
    pub source: ProjectSource,
    pub services: Vec<ComposeService>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ProjectSource {
    Engine,
    LocalFile(String),
}

pub fn discover_projects(context: &str, containers: Vec<ComposeContainer>) -> Vec<ComposeProject> {
    let mut grouped: BTreeMap<String, BTreeMap<String, Vec<ComposeContainer>>> = BTreeMap::new();
    for container in containers {
        grouped
            .entry(
                container
                    .name
                    .split_once('/')
                    .map_or_else(|| "unknown".to_string(), |(project, _)| project.to_string()),
            )
            .or_default()
            .entry(container.service.clone())
            .or_default()
            .push(container);
    }
    grouped
        .into_iter()
        .map(|(name, services)| ComposeProject {
            context: context.to_string(),
            name,
            source: ProjectSource::Engine,
            services: services
                .into_iter()
                .map(|(name, containers)| ComposeService {
                    name,
                    containers,
                    depends_on: Vec::new(),
                })
                .collect(),
        })
        .collect()
}

pub fn compose_project_name(labels: &std::collections::HashMap<String, String>) -> Option<&str> {
    labels.get("com.docker.compose.project").map(String::as_str)
}

pub fn compose_service_name(labels: &std::collections::HashMap<String, String>) -> Option<&str> {
    labels.get("com.docker.compose.service").map(String::as_str)
}

#[allow(dead_code)]
pub fn registered_project(
    context: &str,
    path: impl Into<String>,
    name: impl Into<String>,
) -> ComposeProject {
    ComposeProject {
        context: context.to_string(),
        name: name.into(),
        source: ProjectSource::LocalFile(path.into()),
        services: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ComposeContainer, ComposeService, ProjectSource, discover_projects, registered_project,
    };
    use std::time::{Duration, UNIX_EPOCH};

    fn container(project: &str, service: &str, id: &str) -> ComposeContainer {
        ComposeContainer {
            id: id.into(),
            service: service.into(),
            name: format!("{project}/{id}"),
            state: "running".into(),
            status: "Up".into(),
            image: "demo:latest".into(),
            ports: String::new(),
            health: "no-check".into(),
            restarts: 0,
            started_at: None,
        }
    }

    #[test]
    fn groups_by_compose_project_and_service() {
        let projects = discover_projects(
            "remote",
            vec![
                container("backend", "api", "a"),
                container("backend", "api", "b"),
            ],
        );
        assert_eq!(projects[0].context, "remote");
        assert_eq!(projects[0].services[0].containers.len(), 2);
        assert_eq!(projects[0].source, ProjectSource::Engine);
    }

    #[test]
    fn selection_identity_includes_context_and_project_name() {
        let projects = discover_projects("ctx", vec![container("backend", "api", "a")]);
        assert_eq!(
            (projects[0].context.as_str(), projects[0].name.as_str()),
            ("ctx", "backend")
        );
    }

    #[test]
    fn registered_projects_are_marked_as_local_files() {
        let project = registered_project("ctx", "project files/compose.yml", "backend");
        assert_eq!(
            project.source,
            ProjectSource::LocalFile("project files/compose.yml".into())
        );
    }

    #[test]
    fn service_status_aggregates_replicas_without_inferencing_health() {
        let mut first = container("backend", "api", "a");
        first.health = "healthy".into();
        first.restarts = 2;
        first.started_at = Some("1970-01-01T00:00:00Z".into());
        let mut second = container("backend", "api", "b");
        second.state = "exited".into();
        second.health = "unhealthy".into();
        second.restarts = 3;
        let service = ComposeService {
            name: "api".into(),
            containers: vec![first, second],
            depends_on: Vec::new(),
        };

        assert_eq!(service.status(), "partial");
        assert_eq!(service.health(), "unhealthy");
        assert_eq!(service.replicas(), "2");
        assert_eq!(service.restarts(), 5);
        assert_eq!(service.uptime(UNIX_EPOCH + Duration::from_secs(65)), "1m");
    }

    #[test]
    fn service_without_healthchecks_is_explicit() {
        let service = ComposeService {
            name: "redis".into(),
            containers: vec![container("backend", "redis", "redis")],
            depends_on: Vec::new(),
        };
        assert_eq!(service.health(), "no-check");
    }
}
