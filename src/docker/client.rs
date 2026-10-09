use bollard::query_parameters::StatsOptionsBuilder;
use bollard::{Docker, query_parameters::ListContainersOptionsBuilder};
use futures_util::StreamExt;
use std::fmt;
use tokio::time::{Duration, timeout};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineStatus {
    Connected,
    Disconnected(String),
}

impl EngineStatus {
    pub fn message(&self) -> String {
        match self {
            Self::Connected => "connected".to_string(),
            Self::Disconnected(reason) => format!("disconnected: {}", reason),
        }
    }
}

impl fmt::Display for EngineStatus {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message())
    }
}

pub struct EngineConnection {
    client: Option<Docker>,
    status: EngineStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContainerRow {
    pub id: String,
    pub name: String,
    pub image: String,
    pub state: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ContainerStats {
    pub id: String,
    pub cpu_percent: f64,
    pub memory_usage: u64,
    pub memory_limit: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DashboardData {
    pub engine_version: String,
    pub containers_total: i64,
    pub containers_running: i64,
    pub containers_paused: i64,
    pub containers_stopped: i64,
    pub selected_stats: Option<ContainerStats>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerLifecycle {
    Restart,
    Pause,
    Unpause,
    Remove,
}

impl ContainerLifecycle {
    pub fn label(self) -> &'static str {
        match self {
            Self::Restart => "restart",
            Self::Pause => "pause",
            Self::Unpause => "unpause",
            Self::Remove => "remove",
        }
    }
}

pub async fn list_containers(client: &Docker) -> Result<Vec<ContainerRow>, String> {
    let options = ListContainersOptionsBuilder::new().all(true).build();
    let containers = client
        .list_containers(Some(options))
        .await
        .map_err(|error| error.to_string())?;

    Ok(containers
        .into_iter()
        .map(|container| ContainerRow {
            id: container.id.unwrap_or_else(|| "-".to_string()),
            name: container
                .names
                .unwrap_or_default()
                .into_iter()
                .next()
                .map(|name| name.trim_start_matches('/').to_string())
                .unwrap_or_else(|| "-".to_string()),
            image: container.image.unwrap_or_else(|| "-".to_string()),
            state: container
                .state
                .map(|state| format!("{state:?}"))
                .unwrap_or_else(|| "unknown".to_string()),
            status: container.status.unwrap_or_else(|| "-".to_string()),
        })
        .collect())
}

pub async fn inspect_container(client: &Docker, id: &str) -> Result<Vec<String>, String> {
    let container = client
        .inspect_container(
            id,
            None::<bollard::query_parameters::InspectContainerOptions>,
        )
        .await
        .map_err(|error| error.to_string())?;
    let state = container
        .state
        .and_then(|state| state.status)
        .map(|state| state.to_string())
        .unwrap_or_else(|| "unknown".to_string());
    let image = container
        .config
        .and_then(|config| config.image)
        .unwrap_or_else(|| "unknown".to_string());
    let name = container
        .name
        .map(|name| name.trim_start_matches('/').to_string())
        .unwrap_or_else(|| id.to_string());

    Ok(vec![
        format!(
            "[inspect] id: {}",
            container.id.unwrap_or_else(|| id.to_string())
        ),
        format!("[inspect] name: {}", name),
        format!("[inspect] image: {}", image),
        format!("[inspect] state: {}", state),
        String::new(),
    ])
}

pub async fn apply_container_lifecycle(
    client: &Docker,
    id: &str,
    operation: ContainerLifecycle,
) -> Result<Vec<String>, String> {
    let result = match operation {
        ContainerLifecycle::Restart => {
            client
                .restart_container(
                    id,
                    None::<bollard::query_parameters::RestartContainerOptions>,
                )
                .await
        }
        ContainerLifecycle::Pause => client.pause_container(id).await,
        ContainerLifecycle::Unpause => client.unpause_container(id).await,
        ContainerLifecycle::Remove => {
            client
                .remove_container(
                    id,
                    None::<bollard::query_parameters::RemoveContainerOptions>,
                )
                .await
        }
    };
    result
        .map(|_| {
            vec![
                format!("[docker] {} container {}", operation.label(), id),
                String::new(),
            ]
        })
        .map_err(|error| error.to_string())
}

pub async fn dashboard_data(
    client: &Docker,
    selected_id: Option<&str>,
) -> Result<DashboardData, String> {
    let info = client.info().await.map_err(|error| error.to_string())?;
    let selected_stats = if let Some(id) = selected_id {
        let options = StatsOptionsBuilder::new()
            .stream(false)
            .one_shot(true)
            .build();
        let mut stream = client.stats(id, Some(options));
        match stream.next().await {
            Some(Ok(stats)) => {
                let cpu_total = stats
                    .cpu_stats
                    .as_ref()
                    .and_then(|stats| stats.cpu_usage.as_ref())
                    .and_then(|usage| usage.total_usage)
                    .unwrap_or(0);
                let pre_cpu_total = stats
                    .precpu_stats
                    .as_ref()
                    .and_then(|stats| stats.cpu_usage.as_ref())
                    .and_then(|usage| usage.total_usage)
                    .unwrap_or(0);
                let system_total = stats
                    .cpu_stats
                    .as_ref()
                    .and_then(|stats| stats.system_cpu_usage)
                    .unwrap_or(0);
                let pre_system_total = stats
                    .precpu_stats
                    .as_ref()
                    .and_then(|stats| stats.system_cpu_usage)
                    .unwrap_or(0);
                let online_cpus = stats
                    .cpu_stats
                    .as_ref()
                    .and_then(|stats| stats.online_cpus)
                    .unwrap_or(1);
                let cpu_percent = if system_total > pre_system_total {
                    (cpu_total.saturating_sub(pre_cpu_total) as f64
                        / (system_total - pre_system_total) as f64)
                        * online_cpus as f64
                        * 100.0
                } else {
                    0.0
                };
                let memory = stats.memory_stats.unwrap_or_default();
                Some(ContainerStats {
                    id: id.to_string(),
                    cpu_percent,
                    memory_usage: memory.usage.unwrap_or(0),
                    memory_limit: memory.limit.unwrap_or(0),
                })
            }
            Some(Err(error)) => return Err(error.to_string()),
            None => None,
        }
    } else {
        None
    };

    Ok(DashboardData {
        engine_version: info.server_version.unwrap_or_else(|| "unknown".to_string()),
        containers_total: info.containers.unwrap_or(0),
        containers_running: info.containers_running.unwrap_or(0),
        containers_paused: info.containers_paused.unwrap_or(0),
        containers_stopped: info.containers_stopped.unwrap_or(0),
        selected_stats,
    })
}

impl EngineConnection {
    pub async fn connect() -> Self {
        let client = match Docker::connect_with_local_defaults() {
            Ok(client) => client,
            Err(error) => {
                return Self {
                    client: None,
                    status: EngineStatus::Disconnected(error.to_string()),
                };
            }
        };

        let status = match timeout(Duration::from_secs(2), client.ping()).await {
            Ok(Ok(_)) => EngineStatus::Connected,
            Ok(Err(error)) => EngineStatus::Disconnected(error.to_string()),
            Err(_) => EngineStatus::Disconnected("connection timed out".to_string()),
        };
        let client = if status == EngineStatus::Connected {
            Some(client)
        } else {
            None
        };

        Self { client, status }
    }

    pub fn status(&self) -> &EngineStatus {
        &self.status
    }

    pub fn status_message(&self) -> String {
        self.status.message()
    }

    pub fn into_client(self) -> Option<Docker> {
        self.client
    }
}

#[cfg(test)]
mod tests {
    use super::EngineStatus;

    #[test]
    fn status_messages_are_actionable() {
        assert_eq!(EngineStatus::Connected.message(), "connected");
        assert_eq!(
            EngineStatus::Disconnected("daemon unavailable".to_string()).message(),
            "disconnected: daemon unavailable"
        );
    }
}
