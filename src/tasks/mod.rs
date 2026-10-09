use crate::{
    docker::{CommandSpec, run_command, run_stop_all},
    security::Mutation,
};
use tokio::{sync::mpsc, task::JoinHandle};

pub enum TaskRequest {
    Command {
        spec: CommandSpec,
        mutation: Mutation,
    },
    StopAll,
}

impl TaskRequest {
    pub fn mutation(&self) -> Mutation {
        match self {
            Self::Command { mutation, .. } => *mutation,
            Self::StopAll => Mutation::Destructive,
        }
    }

    pub fn description(&self) -> String {
        match self {
            Self::Command { spec, .. } => spec.display(),
            Self::StopAll => "detener todos los contenedores".to_string(),
        }
    }
}

pub enum TaskEvent {
    Started { id: u64 },
    Progress { id: u64, message: String },
    Finished { id: u64, lines: Vec<String> },
}

pub struct TaskManager {
    sender: mpsc::Sender<TaskEvent>,
    receiver: mpsc::Receiver<TaskEvent>,
    active: Option<(u64, JoinHandle<()>)>,
    next_id: u64,
}

impl TaskManager {
    pub fn new(capacity: usize) -> Self {
        let (sender, receiver) = mpsc::channel(capacity);
        Self {
            sender,
            receiver,
            active: None,
            next_id: 1,
        }
    }

    pub fn spawn(&mut self, request: TaskRequest) -> bool {
        if self.active.is_some() {
            return false;
        }

        let id = self.next_id;
        self.next_id += 1;
        let sender = self.sender.clone();
        let handle = tokio::spawn(async move {
            if sender.send(TaskEvent::Started { id }).await.is_err() {
                return;
            }

            let progress = match &request {
                TaskRequest::Command { spec, .. } => format!("ejecutando: {}", spec.display()),
                TaskRequest::StopAll => "deteniendo contenedores activos".to_string(),
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

            let lines = match request {
                TaskRequest::Command { spec, .. } => run_command(spec).await.lines,
                TaskRequest::StopAll => run_stop_all()
                    .await
                    .into_iter()
                    .flat_map(|result| result.lines)
                    .collect(),
            };

            let _ = sender.send(TaskEvent::Finished { id, lines }).await;
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
    use crate::{docker::CommandSpec, security::Mutation};

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
}
