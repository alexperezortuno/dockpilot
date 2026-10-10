use crate::{
    docker::{
        CommandSpec,
        client::{
            ContainerLifecycle, ContainerRow, DashboardData, HealthSnapshot, ImageRow, NetworkRow,
            VolumeRow, apply_container_lifecycle, container_health, dashboard_data,
            inspect_container, list_containers, list_images, list_networks, list_volumes,
            start_all_containers,
        },
        metrics::container_stats,
        run_command, run_stop_all,
    },
    security::Mutation,
};
use bollard::query_parameters::EventsOptions;
use bollard::query_parameters::LogsOptionsBuilder;
use futures_util::StreamExt;
use tokio::{sync::mpsc, task::JoinHandle};

pub enum TaskRequest {
    Command {
        spec: CommandSpec,
        mutation: Mutation,
    },
    StopAll,
    StartAll,
    ListContainers,
    ListImages,
    ListNetworks,
    ListVolumes,
    InspectContainer {
        id: String,
    },
    ContainerLifecycle {
        id: String,
        operation: ContainerLifecycle,
    },
    ContainerLogs {
        id: String,
        follow: bool,
        timestamps: bool,
    },
    Dashboard {
        selected_id: Option<String>,
    },
    Metrics {
        id: String,
    },
    Health {
        id: String,
    },
    DiskUsage {
        preview: bool,
    },
    Events,
}

impl TaskRequest {
    pub fn mutation(&self) -> Mutation {
        match self {
            Self::Command { mutation, .. } => *mutation,
            Self::StopAll => Mutation::Destructive,
            Self::StartAll => Mutation::Mutating,
            Self::ListContainers => Mutation::ReadOnly,
            Self::ListImages => Mutation::ReadOnly,
            Self::ListNetworks | Self::ListVolumes => Mutation::ReadOnly,
            Self::InspectContainer { .. } => Mutation::ReadOnly,
            Self::ContainerLifecycle { operation, .. } => {
                if *operation == ContainerLifecycle::Remove {
                    Mutation::Destructive
                } else {
                    Mutation::Mutating
                }
            }
            Self::ContainerLogs { .. } => Mutation::ReadOnly,
            Self::Dashboard { .. } => Mutation::ReadOnly,
            Self::Metrics { .. } => Mutation::ReadOnly,
            Self::Health { .. } => Mutation::ReadOnly,
            Self::DiskUsage { .. } => Mutation::ReadOnly,
            Self::Events => Mutation::ReadOnly,
        }
    }

    pub fn description(&self) -> String {
        match self {
            Self::Command { spec, .. } => spec.display(),
            Self::StopAll => "stop all containers".to_string(),
            Self::StartAll => "start all containers".to_string(),
            Self::ListContainers => "list containers".to_string(),
            Self::ListImages => "list images".to_string(),
            Self::ListNetworks => "list networks".to_string(),
            Self::ListVolumes => "list volumes".to_string(),
            Self::InspectContainer { id } => format!("inspect container {}", id),
            Self::ContainerLifecycle { id, operation } => {
                format!("{} container {}", operation.label(), id)
            }
            Self::ContainerLogs {
                id,
                follow,
                timestamps,
            } => {
                format!(
                    "logs {}{}{}",
                    id,
                    if *follow { " (follow)" } else { "" },
                    if *timestamps { " (timestamps)" } else { "" }
                )
            }
            Self::Dashboard { .. } => "update dashboard".to_string(),
            Self::Metrics { id } => format!("sample metrics for {}", id),
            Self::Health { id } => format!("inspect health for {}", id),
            Self::DiskUsage { preview } => if *preview {
                "preview cleanup"
            } else {
                "inspect disk usage"
            }
            .to_string(),
            Self::Events => "stream Docker events".to_string(),
        }
    }
}

pub enum TaskEvent {
    Started {
        id: u64,
        description: String,
    },
    Progress {
        id: u64,
        message: String,
    },
    Finished {
        id: u64,
        lines: Vec<String>,
    },
    Failed {
        id: u64,
        message: String,
    },
    Cancelled {
        id: u64,
    },
    Containers {
        id: u64,
        containers: Vec<ContainerRow>,
    },
    Images {
        id: u64,
        images: Vec<ImageRow>,
    },
    Networks {
        id: u64,
        networks: Vec<NetworkRow>,
    },
    Volumes {
        id: u64,
        volumes: Vec<VolumeRow>,
    },
    LogLine {
        line: String,
    },
    Dashboard {
        id: u64,
        data: DashboardData,
    },
    Metrics {
        id: u64,
        container_id: String,
        sample: crate::app::monitoring::RawSample,
    },
    Health {
        id: u64,
        snapshot: HealthSnapshot,
    },
    EventLine {
        line: String,
        alert: Option<String>,
    },
}

pub struct TaskManager {
    sender: mpsc::Sender<TaskEvent>,
    receiver: mpsc::Receiver<TaskEvent>,
    active: Option<(u64, JoinHandle<()>)>,
    client: Option<bollard::Docker>,
    next_id: u64,
}

impl TaskManager {
    pub fn new(capacity: usize) -> Self {
        let (sender, receiver) = mpsc::channel(capacity);
        Self {
            sender,
            receiver,
            active: None,
            client: None,
            next_id: 1,
        }
    }

    pub fn set_client(&mut self, client: Option<bollard::Docker>) {
        self.client = client;
    }

    pub fn has_client(&self) -> bool {
        self.client.is_some()
    }

    pub fn is_idle(&self) -> bool {
        self.active.is_none()
    }

    pub fn spawn(&mut self, request: TaskRequest) -> bool {
        if self.active.is_some() {
            return false;
        }

        let id = self.next_id;
        self.next_id += 1;
        let sender = self.sender.clone();
        let client = self.client.clone();
        let handle = tokio::spawn(async move {
            let description = request.description();
            if sender
                .send(TaskEvent::Started { id, description })
                .await
                .is_err()
            {
                return;
            }

            let progress = match &request {
                TaskRequest::Command { spec, .. } => format!("executing: {}", spec.display()),
                TaskRequest::StopAll => "stopping active containers".to_string(),
                TaskRequest::StartAll => "starting stopped containers".to_string(),
                TaskRequest::ListContainers => "querying containers".to_string(),
                TaskRequest::ListImages => "querying images".to_string(),
                TaskRequest::ListNetworks => "querying networks".to_string(),
                TaskRequest::ListVolumes => "querying volumes".to_string(),
                TaskRequest::InspectContainer { id } => format!("inspecting {}", id),
                TaskRequest::ContainerLifecycle { id, operation } => {
                    format!("{} container {}", operation.label(), id)
                }
                TaskRequest::ContainerLogs {
                    id,
                    follow,
                    timestamps,
                } => {
                    format!(
                        "logs {}{}{}",
                        id,
                        if *follow { " (follow)" } else { "" },
                        if *timestamps { " (timestamps)" } else { "" }
                    )
                }
                TaskRequest::Dashboard { .. } => "updating dashboard".to_string(),
                TaskRequest::Metrics { id } => format!("sampling metrics for {}", id),
                TaskRequest::Health { id } => format!("inspecting health for {}", id),
                TaskRequest::DiskUsage { preview } => if *preview {
                    "building cleanup preview"
                } else {
                    "querying disk usage"
                }
                .to_string(),
                TaskRequest::Events => "streaming Docker events".to_string(),
            };
            if sender
                .send(TaskEvent::Progress {
                    id,
                    message: progress,
                })
                .await
                .is_err()
            {
                return;
            }

            match request {
                TaskRequest::Command { spec, .. } => {
                    let result = run_command(spec).await;
                    if result.success {
                        let _ = sender
                            .send(TaskEvent::Finished {
                                id,
                                lines: result.lines,
                            })
                            .await;
                    } else {
                        let _ = sender
                            .send(TaskEvent::Failed {
                                id,
                                message: "command exited unsuccessfully".to_string(),
                            })
                            .await;
                    }
                }
                TaskRequest::StopAll => {
                    let results = run_stop_all().await;
                    let success = results.iter().all(|result| result.success);
                    let lines = results
                        .into_iter()
                        .flat_map(|result| result.lines)
                        .collect();
                    if success {
                        let _ = sender.send(TaskEvent::Finished { id, lines }).await;
                    } else {
                        let _ = sender
                            .send(TaskEvent::Failed {
                                id,
                                message: "stop-all command failed".to_string(),
                            })
                            .await;
                    }
                }
                TaskRequest::StartAll => match client {
                    Some(client) => match start_all_containers(&client).await {
                        Ok(lines) => {
                            let _ = sender.send(TaskEvent::Finished { id, lines }).await;
                        }
                        Err(error) => {
                            let _ = sender
                                .send(TaskEvent::Finished {
                                    id,
                                    lines: vec![
                                        format!("[docker] start all failed: {}", error),
                                        String::new(),
                                    ],
                                })
                                .await;
                        }
                    },
                    None => {
                        let _ = sender
                            .send(TaskEvent::Finished {
                                id,
                                lines: vec![
                                    "[docker] Engine disconnected; cannot start containers"
                                        .to_string(),
                                    String::new(),
                                ],
                            })
                            .await;
                    }
                },
                TaskRequest::ListContainers => match client {
                    Some(client) => match list_containers(&client).await {
                        Ok(containers) => {
                            let _ = sender.send(TaskEvent::Containers { id, containers }).await;
                        }
                        Err(error) => {
                            let _ = sender
                                .send(TaskEvent::Finished {
                                    id,
                                    lines: vec![
                                        format!("[docker] error listing containers: {}", error),
                                        String::new(),
                                    ],
                                })
                                .await;
                        }
                    },
                    None => {
                        let _ = sender
                            .send(TaskEvent::Finished {
                                id,
                                lines: vec![
                                    "[docker] Engine disconnected; cannot list containers"
                                        .to_string(),
                                    String::new(),
                                ],
                            })
                            .await;
                    }
                },
                TaskRequest::ListImages => match client {
                    Some(client) => match list_images(&client).await {
                        Ok(images) => {
                            let _ = sender.send(TaskEvent::Images { id, images }).await;
                        }
                        Err(error) => {
                            let _ = sender
                                .send(TaskEvent::Finished {
                                    id,
                                    lines: vec![
                                        format!("[docker] image list failed: {}", error),
                                        String::new(),
                                    ],
                                })
                                .await;
                        }
                    },
                    None => {
                        let _ = sender
                            .send(TaskEvent::Finished {
                                id,
                                lines: vec![
                                    "[docker] Engine disconnected; cannot list images".to_string(),
                                    String::new(),
                                ],
                            })
                            .await;
                    }
                },
                TaskRequest::ListNetworks => match client {
                    Some(client) => match list_networks(&client).await {
                        Ok(networks) => {
                            let _ = sender.send(TaskEvent::Networks { id, networks }).await;
                        }
                        Err(error) => {
                            let _ = sender
                                .send(TaskEvent::Finished {
                                    id,
                                    lines: vec![
                                        format!("[docker] network list failed: {}", error),
                                        String::new(),
                                    ],
                                })
                                .await;
                        }
                    },
                    None => {
                        let _ = sender
                            .send(TaskEvent::Finished {
                                id,
                                lines: vec![
                                    "[docker] Engine disconnected; cannot list networks"
                                        .to_string(),
                                    String::new(),
                                ],
                            })
                            .await;
                    }
                },
                TaskRequest::ListVolumes => match client {
                    Some(client) => match list_volumes(&client).await {
                        Ok(volumes) => {
                            let _ = sender.send(TaskEvent::Volumes { id, volumes }).await;
                        }
                        Err(error) => {
                            let _ = sender
                                .send(TaskEvent::Finished {
                                    id,
                                    lines: vec![
                                        format!("[docker] volume list failed: {}", error),
                                        String::new(),
                                    ],
                                })
                                .await;
                        }
                    },
                    None => {
                        let _ = sender
                            .send(TaskEvent::Finished {
                                id,
                                lines: vec![
                                    "[docker] Engine disconnected; cannot list volumes".to_string(),
                                    String::new(),
                                ],
                            })
                            .await;
                    }
                },
                TaskRequest::InspectContainer { id: container_id } => match client {
                    Some(client) => match inspect_container(&client, &container_id).await {
                        Ok(lines) => {
                            let _ = sender.send(TaskEvent::Finished { id, lines }).await;
                        }
                        Err(error) => {
                            let _ = sender
                                .send(TaskEvent::Finished {
                                    id,
                                    lines: vec![
                                        format!("[docker] inspect failed: {}", error),
                                        String::new(),
                                    ],
                                })
                                .await;
                        }
                    },
                    None => {
                        let _ = sender
                            .send(TaskEvent::Finished {
                                id,
                                lines: vec![
                                    "[docker] Engine disconnected; cannot inspect container"
                                        .to_string(),
                                    String::new(),
                                ],
                            })
                            .await;
                    }
                },
                TaskRequest::ContainerLifecycle {
                    id: container_id,
                    operation,
                } => match client {
                    Some(client) => {
                        match apply_container_lifecycle(&client, &container_id, operation).await {
                            Ok(lines) => {
                                let _ = sender.send(TaskEvent::Finished { id, lines }).await;
                            }
                            Err(error) => {
                                let _ = sender
                                    .send(TaskEvent::Finished {
                                        id,
                                        lines: vec![
                                            format!("[docker] lifecycle failed: {}", error),
                                            String::new(),
                                        ],
                                    })
                                    .await;
                            }
                        }
                    }
                    None => {
                        let _ = sender
                            .send(TaskEvent::Finished {
                                id,
                                lines: vec![
                                    "[docker] Engine disconnected; cannot change container"
                                        .to_string(),
                                    String::new(),
                                ],
                            })
                            .await;
                    }
                },
                TaskRequest::ContainerLogs {
                    id: container_id,
                    follow,
                    timestamps,
                } => match client {
                    Some(client) => {
                        let options = LogsOptionsBuilder::new()
                            .follow(follow)
                            .stdout(true)
                            .stderr(true)
                            .timestamps(timestamps)
                            .tail("200")
                            .build();
                        let mut stream = client.logs(&container_id, Some(options));
                        while let Some(result) = stream.next().await {
                            match result {
                                Ok(output) => {
                                    for line in output.to_string().lines() {
                                        if sender
                                            .send(TaskEvent::LogLine {
                                                line: line.to_string(),
                                            })
                                            .await
                                            .is_err()
                                        {
                                            return;
                                        }
                                    }
                                }
                                Err(error) => {
                                    let _ = sender
                                        .send(TaskEvent::Finished {
                                            id,
                                            lines: vec![
                                                format!("[docker] logs failed: {}", error),
                                                String::new(),
                                            ],
                                        })
                                        .await;
                                    return;
                                }
                            }
                        }
                        let _ = sender
                            .send(TaskEvent::Finished {
                                id,
                                lines: Vec::new(),
                            })
                            .await;
                    }
                    None => {
                        let _ = sender
                            .send(TaskEvent::Finished {
                                id,
                                lines: vec![
                                    "[docker] Engine disconnected; cannot stream logs".to_string(),
                                    String::new(),
                                ],
                            })
                            .await;
                    }
                },
                TaskRequest::Dashboard { selected_id } => match client {
                    Some(client) => match dashboard_data(&client, selected_id.as_deref()).await {
                        Ok(data) => {
                            let _ = sender.send(TaskEvent::Dashboard { id, data }).await;
                        }
                        Err(error) => {
                            let _ = sender
                                .send(TaskEvent::Finished {
                                    id,
                                    lines: vec![
                                        format!("[docker] dashboard failed: {}", error),
                                        String::new(),
                                    ],
                                })
                                .await;
                        }
                    },
                    None => {
                        let _ = sender
                            .send(TaskEvent::Finished {
                                id,
                                lines: vec![
                                    "[docker] Engine disconnected; dashboard unavailable"
                                        .to_string(),
                                    String::new(),
                                ],
                            })
                            .await;
                    }
                },
                TaskRequest::Metrics { id: container_id } => match client {
                    Some(client) => match container_stats(&client, &container_id).await {
                        Ok(sample) => {
                            let _ = sender
                                .send(TaskEvent::Metrics {
                                    id,
                                    container_id,
                                    sample,
                                })
                                .await;
                        }
                        Err(error) => {
                            let _ = sender
                                .send(TaskEvent::Failed {
                                    id,
                                    message: format!("metrics failed: {}", error),
                                })
                                .await;
                        }
                    },
                    None => {
                        let _ = sender
                            .send(TaskEvent::Failed {
                                id,
                                message: "Docker Engine disconnected; metrics unavailable"
                                    .to_string(),
                            })
                            .await;
                    }
                },
                TaskRequest::Health { id: container_id } => match client {
                    Some(client) => match container_health(&client, &container_id).await {
                        Ok(snapshot) => {
                            let _ = sender.send(TaskEvent::Health { id, snapshot }).await;
                        }
                        Err(error) => {
                            let _ = sender
                                .send(TaskEvent::Failed {
                                    id,
                                    message: format!("health inspection failed: {}", error),
                                })
                                .await;
                        }
                    },
                    None => {
                        let _ = sender
                            .send(TaskEvent::Failed {
                                id,
                                message: "Docker Engine disconnected; health unavailable"
                                    .to_string(),
                            })
                            .await;
                    }
                },
                TaskRequest::DiskUsage { preview } => match client {
                    Some(client) => {
                        match crate::docker::client::disk_usage_lines(&client, preview).await {
                            Ok(lines) => {
                                let _ = sender.send(TaskEvent::Finished { id, lines }).await;
                            }
                            Err(error) => {
                                let _ = sender
                                    .send(TaskEvent::Finished {
                                        id,
                                        lines: vec![
                                            format!("[docker] disk usage failed: {}", error),
                                            String::new(),
                                        ],
                                    })
                                    .await;
                            }
                        }
                    }
                    None => {
                        let _ = sender
                            .send(TaskEvent::Finished {
                                id,
                                lines: vec![
                                    "[docker] Engine disconnected; disk usage unavailable"
                                        .to_string(),
                                    String::new(),
                                ],
                            })
                            .await;
                    }
                },
                TaskRequest::Events => match client {
                    Some(client) => {
                        let mut stream = client.events(None::<EventsOptions>);
                        while let Some(result) = stream.next().await {
                            match result {
                                Ok(event) => {
                                    let event_type = event
                                        .typ
                                        .map(|value| value.to_string())
                                        .unwrap_or_else(|| "unknown".to_string());
                                    let action =
                                        event.action.unwrap_or_else(|| "unknown".to_string());
                                    let id = event
                                        .actor
                                        .and_then(|actor| actor.id)
                                        .unwrap_or_else(|| "-".to_string());
                                    let line = format!("{} {} {}", event_type, action, id);
                                    let lower = line.to_lowercase();
                                    let alert = ["die", "kill", "oom", "destroy", "unhealthy"]
                                        .iter()
                                        .any(|marker| lower.contains(marker))
                                        .then(|| format!("[alert] {}", line));
                                    if sender
                                        .send(TaskEvent::EventLine { line, alert })
                                        .await
                                        .is_err()
                                    {
                                        return;
                                    }
                                }
                                Err(error) => {
                                    let _ = sender
                                        .send(TaskEvent::Finished {
                                            id,
                                            lines: vec![
                                                format!("[docker] events failed: {}", error),
                                                String::new(),
                                            ],
                                        })
                                        .await;
                                    return;
                                }
                            }
                        }
                        let _ = sender
                            .send(TaskEvent::Finished {
                                id,
                                lines: Vec::new(),
                            })
                            .await;
                    }
                    None => {
                        let _ = sender
                            .send(TaskEvent::Finished {
                                id,
                                lines: vec![
                                    "[docker] Engine disconnected; events unavailable".to_string(),
                                    String::new(),
                                ],
                            })
                            .await;
                    }
                },
            }
        });

        self.active = Some((id, handle));
        true
    }

    pub fn try_next(&mut self) -> Option<TaskEvent> {
        self.receiver.try_recv().ok()
    }

    pub fn complete(&mut self, id: u64) {
        if self
            .active
            .as_ref()
            .is_some_and(|(active_id, _)| *active_id == id)
        {
            self.active = None;
        }
    }

    pub fn cancel(&mut self) -> bool {
        if let Some((id, handle)) = self.active.take() {
            let _ = self.sender.try_send(TaskEvent::Cancelled { id });
            handle.abort();
            true
        } else {
            false
        }
    }
}

impl Drop for TaskManager {
    fn drop(&mut self) {
        self.cancel();
    }
}

#[cfg(test)]
mod tests {
    use super::{TaskManager, TaskRequest};
    use crate::{
        docker::{CommandSpec, client::ContainerLifecycle},
        security::Mutation,
    };

    #[tokio::test]
    async fn manager_allows_one_active_task_and_cancels_it() {
        let mut manager = TaskManager::new(1);

        assert!(manager.spawn(TaskRequest::Command {
            spec: CommandSpec::new("docker"),
            mutation: Mutation::ReadOnly,
        }));
        assert!(!manager.spawn(TaskRequest::Command {
            spec: CommandSpec::new("docker"),
            mutation: Mutation::ReadOnly,
        }));
        assert!(manager.cancel());
        assert!(!manager.cancel());
    }

    #[test]
    fn removing_a_container_is_classified_as_destructive() {
        let request = TaskRequest::ContainerLifecycle {
            id: "container-id".to_string(),
            operation: ContainerLifecycle::Remove,
        };

        assert_eq!(request.mutation(), Mutation::Destructive);
    }
}
