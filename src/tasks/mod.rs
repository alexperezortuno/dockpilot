use crate::{
    docker::{
        CommandSpec,
        client::{
            ContainerLifecycle, ContainerRow, DashboardData, apply_container_lifecycle,
            dashboard_data, inspect_container, list_containers,
        },
        run_command, run_stop_all,
    },
    security::Mutation,
};
use bollard::query_parameters::LogsOptionsBuilder;
use futures_util::StreamExt;
use tokio::{sync::mpsc, task::JoinHandle};

pub enum TaskRequest {
    Command {
        spec: CommandSpec,
        mutation: Mutation,
    },
    StopAll,
    ListContainers,
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
    },
    Dashboard {
        selected_id: Option<String>,
    },
}

impl TaskRequest {
    pub fn mutation(&self) -> Mutation {
        match self {
            Self::Command { mutation, .. } => *mutation,
            Self::StopAll => Mutation::Destructive,
            Self::ListContainers => Mutation::ReadOnly,
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
        }
    }

    pub fn description(&self) -> String {
        match self {
            Self::Command { spec, .. } => spec.display(),
            Self::StopAll => "stop all containers".to_string(),
            Self::ListContainers => "list containers".to_string(),
            Self::InspectContainer { id } => format!("inspect container {}", id),
            Self::ContainerLifecycle { id, operation } => {
                format!("{} container {}", operation.label(), id)
            }
            Self::ContainerLogs { id, follow } => {
                format!("logs {}{}", id, if *follow { " (follow)" } else { "" })
            }
            Self::Dashboard { .. } => "update dashboard".to_string(),
        }
    }
}

pub enum TaskEvent {
    Started {
        id: u64,
    },
    Progress {
        id: u64,
        message: String,
    },
    Finished {
        id: u64,
        lines: Vec<String>,
    },
    Containers {
        id: u64,
        containers: Vec<ContainerRow>,
    },
    LogLine {
        line: String,
    },
    Dashboard {
        id: u64,
        data: DashboardData,
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

    pub fn spawn(&mut self, request: TaskRequest) -> bool {
        if self.active.is_some() {
            return false;
        }

        let id = self.next_id;
        self.next_id += 1;
        let sender = self.sender.clone();
        let client = self.client.clone();
        let handle = tokio::spawn(async move {
            if sender.send(TaskEvent::Started { id }).await.is_err() {
                return;
            }

            let progress = match &request {
                TaskRequest::Command { spec, .. } => format!("executing: {}", spec.display()),
                TaskRequest::StopAll => "stopping active containers".to_string(),
                TaskRequest::ListContainers => "querying containers".to_string(),
                TaskRequest::InspectContainer { id } => format!("inspecting {}", id),
                TaskRequest::ContainerLifecycle { id, operation } => {
                    format!("{} container {}", operation.label(), id)
                }
                TaskRequest::ContainerLogs { id, follow } => {
                    format!("logs {}{}", id, if *follow { " (follow)" } else { "" })
                }
                TaskRequest::Dashboard { .. } => "updating dashboard".to_string(),
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
                    let lines = run_command(spec).await.lines;
                    let _ = sender.send(TaskEvent::Finished { id, lines }).await;
                }
                TaskRequest::StopAll => {
                    let lines = run_stop_all()
                        .await
                        .into_iter()
                        .flat_map(|result| result.lines)
                        .collect();
                    let _ = sender.send(TaskEvent::Finished { id, lines }).await;
                }
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
                } => match client {
                    Some(client) => {
                        let options = LogsOptionsBuilder::new()
                            .follow(follow)
                            .stdout(true)
                            .stderr(true)
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
        if let Some((_, handle)) = self.active.take() {
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
