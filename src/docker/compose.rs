use std::collections::BTreeMap;

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
    use super::{ComposeContainer, ProjectSource, discover_projects, registered_project};

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
}
