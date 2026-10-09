use crate::{
    docker::{
        CommandSpec,
        client::{ContainerLifecycle, ContainerRow, DashboardData, ImageRow},
    },
    security::Mutation,
    tasks::TaskRequest,
};
use ratatui::widgets::ListState;
use std::{collections::VecDeque, path::PathBuf};

const MAX_LOG_LINES: usize = 2_000;

#[derive(Debug, Clone)]
pub enum ContainerAction {
    StartAll,
    Start,
    StopAll,
    Stop,
    Restart,
    ListAll,
    List,
    Logs,
    Create,
    Remove,
    Top,
    Diff,
    Pause,
    Unpause,
    Update,
    Wait,
}

#[derive(Debug, Clone)]
pub enum ImageAction {
    Build,
    Rebuild,
    List,
    Remove,
    Push,
    Pull,
    Save,
    Load,
    History,
}

#[derive(Debug, Clone)]
pub enum NetworkAction {
    List,
    Create,
    Remove,
}

#[derive(Debug, Clone)]
pub enum VolumeAction {
    List,
    Create,
    Remove,
}

#[derive(Debug, Clone)]
pub enum ProjectAction {
    SetFolder,
    ComposeUp,
    ComposeUpProfile,
    ComposeDown,
    ComposeConfig,
}

#[derive(Debug, Clone)]
pub enum MachineAction {
    List,
    Start,
    Stop,
    Env,
    Eval,
    Ip,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Tab {
    Dashboard,
    Container,
    Image,
    Network,
    Volume,
    Project,
    Machine,
    Help,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerSort {
    Name,
    Image,
    State,
}

/// Actions that require a user-supplied parameter.
#[derive(Debug, Clone)]
pub enum PendingAction {
    // Container
    ContainerLogs,
    ContainerStart,
    ContainerStop,
    ContainerTop,
    ContainerDiff,
    ContainerPause,
    ContainerUnpause,
    ContainerUpdate,
    ContainerWait,
    ContainerRemove,
    ContainerCreate,
    ContainerFilter,
    ContainerRestart,
    LogFilter,
    // Image
    ImageRemove,
    ImagePush,
    ImagePull,
    ImageSave,
    ImageLoad,
    ImageHistory,
    ImageBuild,
    ImageRebuild,
    // Network
    NetworkCreate,
    NetworkRemove,
    // Volume
    VolumeCreate,
    VolumeRemove,
    // Project
    ProjectSetFolder,
    ComposeProfileUp,
}

pub struct App {
    pub(crate) current_tab: Tab,
    pub(crate) container_actions: Vec<ContainerAction>,
    pub(crate) image_actions: Vec<ImageAction>,
    pub(crate) images: Vec<ImageRow>,
    pub(crate) image_table_state: ratatui::widgets::TableState,
    pub(crate) network_actions: Vec<NetworkAction>,
    pub(crate) volume_actions: Vec<VolumeAction>,
    pub(crate) project_actions: Vec<ProjectAction>,
    pub(crate) machine_actions: Vec<MachineAction>,
    pub(crate) output_lines: Vec<String>,
    pub(crate) container_list_state: ListState,
    pub(crate) image_list_state: ListState,
    pub(crate) network_list_state: ListState,
    pub(crate) volume_list_state: ListState,
    pub(crate) project_list_state: ListState,
    pub(crate) machine_list_state: ListState,
    pub(crate) output_scroll: u16,
    pub(crate) project_folder: String,
    pub(crate) engine_status: String,
    pub(crate) dashboard: Option<DashboardData>,
    pub(crate) log_lines: VecDeque<String>,
    pub(crate) log_filter: String,
    pub(crate) logs_paused: bool,
    pub(crate) containers: Vec<ContainerRow>,
    pub(crate) container_table_state: ratatui::widgets::TableState,
    pub(crate) container_table_focus: bool,
    pub(crate) container_filter: String,
    pub(crate) container_sort: ContainerSort,
    // Input mode
    pub(crate) input_mode: bool,
    pub(crate) input_buffer: String,
    pub(crate) input_prompt: String,
    pub(crate) pending_action: Option<PendingAction>,
}

impl App {
    #[cfg(test)]
    pub fn new() -> Self {
        let project_folder = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Self::with_project_folder(project_folder)
    }

    pub fn with_project_folder(project_folder: PathBuf) -> Self {
        let project_folder = project_folder.to_string_lossy().to_string();
        let mut app = App {
            current_tab: Tab::Container,
            container_actions: vec![
                ContainerAction::StartAll,
                ContainerAction::Start,
                ContainerAction::StopAll,
                ContainerAction::Stop,
                ContainerAction::Restart,
                ContainerAction::ListAll,
                ContainerAction::List,
                ContainerAction::Logs,
                ContainerAction::Create,
                ContainerAction::Remove,
                ContainerAction::Top,
                ContainerAction::Diff,
                ContainerAction::Pause,
                ContainerAction::Unpause,
                ContainerAction::Update,
                ContainerAction::Wait,
            ],
            image_actions: vec![
                ImageAction::Build,
                ImageAction::Rebuild,
                ImageAction::List,
                ImageAction::Remove,
                ImageAction::Push,
                ImageAction::Pull,
                ImageAction::Save,
                ImageAction::Load,
                ImageAction::History,
            ],
            images: Vec::new(),
            image_table_state: ratatui::widgets::TableState::default(),
            network_actions: vec![
                NetworkAction::List,
                NetworkAction::Create,
                NetworkAction::Remove,
            ],
            volume_actions: vec![
                VolumeAction::List,
                VolumeAction::Create,
                VolumeAction::Remove,
            ],
            project_actions: vec![
                ProjectAction::SetFolder,
                ProjectAction::ComposeUp,
                ProjectAction::ComposeUpProfile,
                ProjectAction::ComposeDown,
                ProjectAction::ComposeConfig,
            ],
            machine_actions: vec![
                MachineAction::List,
                MachineAction::Start,
                MachineAction::Stop,
                MachineAction::Env,
                MachineAction::Eval,
                MachineAction::Ip,
            ],
            output_lines: vec![],
            container_list_state: ListState::default(),
            image_list_state: ListState::default(),
            network_list_state: ListState::default(),
            volume_list_state: ListState::default(),
            project_list_state: ListState::default(),
            machine_list_state: ListState::default(),
            output_scroll: 0,
            project_folder,
            engine_status: "checking Docker Engine".to_string(),
            dashboard: None,
            log_lines: VecDeque::new(),
            log_filter: String::new(),
            logs_paused: false,
            containers: Vec::new(),
            container_table_state: ratatui::widgets::TableState::default(),
            container_table_focus: true,
            container_filter: String::new(),
            container_sort: ContainerSort::Name,
            input_mode: false,
            input_buffer: String::new(),
            input_prompt: String::new(),
            pending_action: None,
        };

        app.container_list_state.select(Some(1));
        app.image_list_state.select(Some(0));
        app.network_list_state.select(Some(0));
        app.volume_list_state.select(Some(0));
        app.project_list_state.select(Some(0));
        app.machine_list_state.select(Some(0));

        app
    }

    pub fn push_output(&mut self, line: impl Into<String>) {
        self.output_lines.push(line.into());
        self.output_scroll = self.output_lines.len() as u16;
    }

    pub fn append_output(&mut self, lines: impl IntoIterator<Item = String>) {
        self.output_lines.extend(lines);
        self.output_scroll = self.output_lines.len() as u16;
    }

    pub fn set_containers(&mut self, containers: Vec<ContainerRow>) {
        self.containers = containers;
        self.container_table_state
            .select(if self.containers.is_empty() {
                None
            } else {
                Some(0)
            });
    }

    pub fn set_images(&mut self, images: Vec<ImageRow>) {
        self.images = images;
        self.image_table_state.select(if self.images.is_empty() {
            None
        } else {
            Some(0)
        });
    }

    pub fn toggle_container_focus(&mut self) {
        self.container_table_focus = !self.container_table_focus;
    }

    pub fn toggle_container_sort(&mut self) {
        self.container_sort = match self.container_sort {
            ContainerSort::Name => ContainerSort::Image,
            ContainerSort::Image => ContainerSort::State,
            ContainerSort::State => ContainerSort::Name,
        };
        self.container_table_state.select(Some(0));
    }

    pub fn start_container_filter(&mut self) {
        self.start_input("Filtro de contenedores:", PendingAction::ContainerFilter);
    }

    pub fn filtered_containers(&self) -> Vec<ContainerRow> {
        let filter = self.container_filter.to_lowercase();
        let mut containers: Vec<_> = self
            .containers
            .iter()
            .filter(|container| {
                filter.is_empty()
                    || [
                        &container.id,
                        &container.name,
                        &container.image,
                        &container.state,
                        &container.status,
                    ]
                    .iter()
                    .any(|value| value.to_lowercase().contains(&filter))
            })
            .cloned()
            .collect();
        containers.sort_by(|left, right| match self.container_sort {
            ContainerSort::Name => left.name.cmp(&right.name),
            ContainerSort::Image => left.image.cmp(&right.image),
            ContainerSort::State => left.state.cmp(&right.state),
        });
        containers
    }

    pub fn set_engine_status(&mut self, status: impl Into<String>) {
        self.engine_status = status.into();
    }

    pub fn set_dashboard(&mut self, dashboard: DashboardData) {
        self.dashboard = Some(dashboard);
    }

    pub fn selected_container_id(&self) -> Option<String> {
        let index = self.container_table_state.selected()?;
        self.filtered_containers()
            .get(index)
            .map(|container| container.id.clone())
    }

    pub fn dashboard_request(&self) -> TaskRequest {
        TaskRequest::Dashboard {
            selected_id: self.selected_container_id(),
        }
    }

    pub fn push_log_line(&mut self, line: String) {
        if self.logs_paused {
            return;
        }
        if self.log_lines.len() == MAX_LOG_LINES {
            self.log_lines.pop_front();
        }
        self.log_lines.push_back(line);
    }

    pub fn toggle_logs_paused(&mut self) {
        self.logs_paused = !self.logs_paused;
    }

    pub fn start_log_filter(&mut self) {
        self.start_input("Filtro de logs:", PendingAction::LogFilter);
    }

    pub fn filtered_log_lines(&self) -> Vec<String> {
        let filter = self.log_filter.to_lowercase();
        self.log_lines
            .iter()
            .filter(|line| filter.is_empty() || line.to_lowercase().contains(&filter))
            .cloned()
            .collect()
    }

    // --- Generic navigation ---
    fn next_in_list(state: &mut ListState, len: usize) {
        if len == 0 {
            return;
        }
        let i = match state.selected() {
            Some(i) => (i + 1) % len,
            None => 0,
        };
        state.select(Some(i));
    }

    fn previous_in_list(state: &mut ListState, len: usize) {
        if len == 0 {
            return;
        }
        let i = match state.selected() {
            Some(i) => {
                if i == 0 {
                    len - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        state.select(Some(i));
    }

    fn next_in_table(state: &mut ratatui::widgets::TableState, len: usize) {
        if len == 0 {
            return;
        }
        let index = match state.selected() {
            Some(index) => (index + 1) % len,
            None => 0,
        };
        state.select(Some(index));
    }

    fn previous_in_table(state: &mut ratatui::widgets::TableState, len: usize) {
        if len == 0 {
            return;
        }
        let index = match state.selected() {
            Some(0) | None => len - 1,
            Some(index) => index - 1,
        };
        state.select(Some(index));
    }

    pub fn next(&mut self) {
        match self.current_tab {
            Tab::Dashboard => {}
            Tab::Container => {
                if self.container_table_focus {
                    let len = self.filtered_containers().len();
                    Self::next_in_table(&mut self.container_table_state, len);
                } else {
                    Self::next_in_list(
                        &mut self.container_list_state,
                        self.container_actions.len(),
                    );
                }
            }
            Tab::Image => Self::next_in_list(&mut self.image_list_state, self.image_actions.len()),
            Tab::Network => {
                Self::next_in_list(&mut self.network_list_state, self.network_actions.len())
            }
            Tab::Volume => {
                Self::next_in_list(&mut self.volume_list_state, self.volume_actions.len())
            }
            Tab::Project => {
                Self::next_in_list(&mut self.project_list_state, self.project_actions.len())
            }
            Tab::Machine => {
                Self::next_in_list(&mut self.machine_list_state, self.machine_actions.len())
            }
            Tab::Help => {}
        }
    }

    pub fn previous(&mut self) {
        match self.current_tab {
            Tab::Dashboard => {}
            Tab::Container => {
                if self.container_table_focus {
                    let len = self.filtered_containers().len();
                    Self::previous_in_table(&mut self.container_table_state, len);
                } else {
                    Self::previous_in_list(
                        &mut self.container_list_state,
                        self.container_actions.len(),
                    );
                }
            }
            Tab::Image => {
                Self::previous_in_list(&mut self.image_list_state, self.image_actions.len())
            }
            Tab::Network => {
                Self::previous_in_list(&mut self.network_list_state, self.network_actions.len())
            }
            Tab::Volume => {
                Self::previous_in_list(&mut self.volume_list_state, self.volume_actions.len())
            }
            Tab::Project => {
                Self::previous_in_list(&mut self.project_list_state, self.project_actions.len())
            }
            Tab::Machine => {
                Self::previous_in_list(&mut self.machine_list_state, self.machine_actions.len())
            }
            Tab::Help => {}
        }
    }

    pub fn next_tab(&mut self) {
        self.current_tab = match self.current_tab {
            Tab::Dashboard => Tab::Container,
            Tab::Container => Tab::Image,
            Tab::Image => Tab::Network,
            Tab::Network => Tab::Volume,
            Tab::Volume => Tab::Project,
            Tab::Project => Tab::Machine,
            Tab::Machine => Tab::Help,
            Tab::Help => Tab::Dashboard,
        };
    }

    pub fn previous_tab(&mut self) {
        self.current_tab = match self.current_tab {
            Tab::Dashboard => Tab::Help,
            Tab::Container => Tab::Dashboard,
            Tab::Image => Tab::Container,
            Tab::Network => Tab::Image,
            Tab::Volume => Tab::Network,
            Tab::Project => Tab::Volume,
            Tab::Machine => Tab::Project,
            Tab::Help => Tab::Machine,
        };
    }

    // --- Input mode ---
    fn start_input(&mut self, prompt: &str, action: PendingAction) -> Option<TaskRequest> {
        self.input_mode = true;
        self.input_buffer.clear();
        self.input_prompt = prompt.to_string();
        self.pending_action = Some(action);
        None
    }

    pub fn cancel_input(&mut self) {
        self.input_mode = false;
        self.input_buffer.clear();
        self.input_prompt.clear();
        self.pending_action = None;
    }

    pub fn confirm_input(&mut self) -> Option<TaskRequest> {
        let value = self.input_buffer.trim().to_string();
        let action = self.pending_action.take();
        self.input_mode = false;
        self.input_buffer.clear();
        self.input_prompt.clear();

        if value.is_empty() {
            self.output_lines
                .push("[entrada cancelada: valor vacío]".to_string());
            return None;
        }

        if let Some(action) = action {
            return self.execute_pending_action(action, &value);
        }

        None
    }

    fn execute_pending_action(
        &mut self,
        action: PendingAction,
        value: &str,
    ) -> Option<TaskRequest> {
        match action {
            // Container
            PendingAction::ContainerStart => {
                self.execute_container_lifecycle(value, ContainerLifecycle::Start)
            }
            PendingAction::ContainerStop => {
                self.execute_container_lifecycle(value, ContainerLifecycle::Stop)
            }
            PendingAction::ContainerLogs => Some(TaskRequest::ContainerLogs {
                id: value.to_string(),
                follow: true,
            }),
            PendingAction::ContainerTop => self.execute_command(
                CommandSpec::new("docker")
                    .args(["container", "top"])
                    .arg(value),
            ),
            PendingAction::ContainerDiff => self.execute_command(
                CommandSpec::new("docker")
                    .args(["container", "diff"])
                    .arg(value),
            ),
            PendingAction::ContainerPause => {
                self.execute_container_lifecycle(value, ContainerLifecycle::Pause)
            }
            PendingAction::ContainerUnpause => {
                self.execute_container_lifecycle(value, ContainerLifecycle::Unpause)
            }
            PendingAction::ContainerUpdate => self.execute_mutating_command(
                CommandSpec::new("docker")
                    .args(["container", "update", "--memory=512m"])
                    .arg(value),
            ),
            PendingAction::ContainerWait => self.execute_command(
                CommandSpec::new("docker")
                    .args(["container", "wait"])
                    .arg(value),
            ),
            PendingAction::ContainerRemove => {
                self.execute_container_lifecycle(value, ContainerLifecycle::Remove)
            }
            PendingAction::ContainerRestart => {
                self.execute_container_lifecycle(value, ContainerLifecycle::Restart)
            }
            PendingAction::ContainerCreate => {
                // value = "image name" e.g. "test nginx"
                let mut parts = value.splitn(2, ' ');
                let name = parts.next().unwrap_or("test");
                let image = parts.next().unwrap_or("nginx");
                self.execute_mutating_command(
                    CommandSpec::new("docker")
                        .args(["run", "-d"])
                        .arg("--name")
                        .arg(name)
                        .args(["-p", "83:83"])
                        .arg(image),
                )
            }
            PendingAction::ContainerFilter => {
                self.container_filter = value.to_string();
                self.container_table_state.select(Some(0));
                None
            }
            PendingAction::LogFilter => {
                self.log_filter = value.to_string();
                None
            }
            // Image
            PendingAction::ImageRemove => self
                .execute_destructive_command(CommandSpec::new("docker").args(["rmi"]).arg(value)),
            PendingAction::ImagePush => {
                self.execute_mutating_command(CommandSpec::new("docker").args(["push"]).arg(value))
            }
            PendingAction::ImagePull => {
                self.execute_mutating_command(CommandSpec::new("docker").args(["pull"]).arg(value))
            }
            PendingAction::ImageSave => self.execute_mutating_command(
                CommandSpec::new("docker")
                    .args(["save"])
                    .arg(value)
                    .stdout_file("image.tar"),
            ),
            PendingAction::ImageLoad => {
                // value = path to the tar file, defaults to image.tar
                self.execute_mutating_command(
                    CommandSpec::new("docker").args(["load"]).stdin_file(value),
                )
            }
            PendingAction::ImageHistory => {
                self.execute_command(CommandSpec::new("docker").args(["history"]).arg(value))
            }
            PendingAction::ImageBuild => {
                // value = "tag context"  p.ej. "myimage ."
                let mut parts = value.splitn(2, ' ');
                let tag = parts.next().unwrap_or("myimage");
                let ctx = parts.next().unwrap_or(".");
                self.execute_mutating_command(
                    CommandSpec::new("docker").args(["build", "-t", tag, ctx]),
                )
            }
            PendingAction::ImageRebuild => {
                let mut parts = value.splitn(2, ' ');
                let tag = parts.next().unwrap_or("myimage");
                let ctx = parts.next().unwrap_or(".");
                self.execute_mutating_command(CommandSpec::new("docker").args([
                    "build",
                    "--no-cache",
                    "-t",
                    tag,
                    ctx,
                ]))
            }
            // Network
            PendingAction::NetworkCreate => self.execute_mutating_command(
                CommandSpec::new("docker")
                    .args(["network", "create"])
                    .arg(value),
            ),
            PendingAction::NetworkRemove => self.execute_destructive_command(
                CommandSpec::new("docker")
                    .args(["network", "rm"])
                    .arg(value),
            ),
            // Volume
            PendingAction::VolumeCreate => self.execute_mutating_command(
                CommandSpec::new("docker")
                    .args(["volume", "create"])
                    .arg(value),
            ),
            PendingAction::VolumeRemove => self.execute_destructive_command(
                CommandSpec::new("docker").args(["volume", "rm"]).arg(value),
            ),
            // Project
            PendingAction::ProjectSetFolder => {
                self.project_folder = value.to_string();
                let folder = self.project_folder.clone();
                self.output_lines
                    .push(format!("[proyecto] carpeta establecida: {}", folder));
                self.execute_command(CommandSpec::new("ls").args(["-la"]).arg(folder))
            }
            PendingAction::ComposeProfileUp => {
                self.execute_compose(&["compose", "--profile", value, "up", "-d"])
            }
        }
    }

    // --- Command executions ---
    fn execute_command(&mut self, command: CommandSpec) -> Option<TaskRequest> {
        Some(TaskRequest::Command {
            spec: command,
            mutation: Mutation::ReadOnly,
        })
    }

    fn execute_mutating_command(&mut self, command: CommandSpec) -> Option<TaskRequest> {
        Some(TaskRequest::Command {
            spec: command,
            mutation: Mutation::Mutating,
        })
    }

    fn execute_destructive_command(&mut self, command: CommandSpec) -> Option<TaskRequest> {
        Some(TaskRequest::Command {
            spec: command,
            mutation: Mutation::Destructive,
        })
    }

    fn execute_container_lifecycle(
        &mut self,
        id: &str,
        operation: ContainerLifecycle,
    ) -> Option<TaskRequest> {
        Some(TaskRequest::ContainerLifecycle {
            id: id.to_string(),
            operation,
        })
    }

    fn execute_compose(&mut self, args: &[&str]) -> Option<TaskRequest> {
        let project_folder = self.project_folder.clone();
        self.execute_mutating_command(
            CommandSpec::new("docker")
                .args(args)
                .current_dir(project_folder),
        )
    }

    fn execute_compose_read_only(&mut self, args: &[&str]) -> Option<TaskRequest> {
        let project_folder = self.project_folder.clone();
        self.execute_command(
            CommandSpec::new("docker")
                .args(args)
                .current_dir(project_folder),
        )
    }

    fn execute_compose_down(&mut self) -> Option<TaskRequest> {
        let project_folder = self.project_folder.clone();
        self.execute_destructive_command(
            CommandSpec::new("docker")
                .args(["compose", "down"])
                .current_dir(project_folder),
        )
    }

    // --- Tab actions ---
    fn run_container_action(&mut self, action: &ContainerAction) -> Option<TaskRequest> {
        match action {
            ContainerAction::StartAll => Some(TaskRequest::StartAll),
            ContainerAction::Start => {
                self.start_input("Contenedor a iniciar:", PendingAction::ContainerStart)
            }
            ContainerAction::StopAll => Some(TaskRequest::StopAll),
            ContainerAction::Stop => {
                self.start_input("Contenedor a detener:", PendingAction::ContainerStop)
            }
            ContainerAction::Restart => {
                self.start_input("Contenedor a reiniciar:", PendingAction::ContainerRestart)
            }
            ContainerAction::ListAll => {
                self.execute_command(CommandSpec::new("docker").args(["ps", "-a"]))
            }
            ContainerAction::List => self.execute_command(CommandSpec::new("docker").args(["ps"])),
            ContainerAction::Logs => {
                self.start_input("Contenedor para ver logs:", PendingAction::ContainerLogs)
            }
            ContainerAction::Create => self.start_input(
                "Nombre e imagen (ej: test nginx):",
                PendingAction::ContainerCreate,
            ),
            ContainerAction::Remove => self.start_input(
                "Contenedor a eliminar (ID o nombre):",
                PendingAction::ContainerRemove,
            ),
            ContainerAction::Top => {
                self.start_input("Contenedor para top:", PendingAction::ContainerTop)
            }
            ContainerAction::Diff => {
                self.start_input("Contenedor para diff:", PendingAction::ContainerDiff)
            }
            ContainerAction::Pause => {
                self.start_input("Contenedor a pausar:", PendingAction::ContainerPause)
            }
            ContainerAction::Unpause => {
                self.start_input("Contenedor a reanudar:", PendingAction::ContainerUnpause)
            }
            ContainerAction::Update => {
                self.start_input("Contenedor a actualizar:", PendingAction::ContainerUpdate)
            }
            ContainerAction::Wait => {
                self.start_input("Contenedor a esperar:", PendingAction::ContainerWait)
            }
        }
    }

    fn run_image_action(&mut self, action: &ImageAction) -> Option<TaskRequest> {
        match action {
            ImageAction::Build => {
                self.start_input("Tag y contexto (ej: myimage .):", PendingAction::ImageBuild)
            }
            ImageAction::Rebuild => self.start_input(
                "Tag y contexto (ej: myimage .):",
                PendingAction::ImageRebuild,
            ),
            ImageAction::List => Some(TaskRequest::ListImages),
            ImageAction::Remove => self.start_input(
                "Imagen a eliminar (ID o nombre):",
                PendingAction::ImageRemove,
            ),
            ImageAction::Push => {
                self.start_input("Imagen a subir (tag):", PendingAction::ImagePush)
            }
            ImageAction::Pull => self.start_input("Imagen a descargar:", PendingAction::ImagePull),
            ImageAction::Save => self.start_input("Imagen a guardar:", PendingAction::ImageSave),
            ImageAction::Load => {
                self.start_input("Ruta del tar (ej: image.tar):", PendingAction::ImageLoad)
            }
            ImageAction::History => {
                self.start_input("Imagen a inspeccionar:", PendingAction::ImageHistory)
            }
        }
    }

    fn run_network_action(&mut self, action: &NetworkAction) -> Option<TaskRequest> {
        match action {
            NetworkAction::List => {
                self.execute_command(CommandSpec::new("docker").args(["network", "ls"]))
            }
            NetworkAction::Create => {
                self.start_input("Nombre de la nueva red:", PendingAction::NetworkCreate)
            }
            NetworkAction::Remove => {
                self.start_input("Nombre de la red a eliminar:", PendingAction::NetworkRemove)
            }
        }
    }

    fn run_volume_action(&mut self, action: &VolumeAction) -> Option<TaskRequest> {
        match action {
            VolumeAction::List => {
                self.execute_command(CommandSpec::new("docker").args(["volume", "ls"]))
            }
            VolumeAction::Create => {
                self.start_input("Nombre del nuevo volumen:", PendingAction::VolumeCreate)
            }
            VolumeAction::Remove => self.start_input(
                "Nombre del volumen a eliminar:",
                PendingAction::VolumeRemove,
            ),
        }
    }

    fn run_project_action(&mut self, action: &ProjectAction) -> Option<TaskRequest> {
        match action {
            ProjectAction::SetFolder => self.start_input(
                "Ruta de la carpeta del proyecto:",
                PendingAction::ProjectSetFolder,
            ),
            ProjectAction::ComposeUp => self.execute_compose(&["compose", "up", "-d"]),
            ProjectAction::ComposeUpProfile => {
                self.start_input("Perfil Compose:", PendingAction::ComposeProfileUp)
            }
            ProjectAction::ComposeDown => self.execute_compose_down(),
            ProjectAction::ComposeConfig => self.execute_compose_read_only(&["compose", "config"]),
        }
    }

    fn run_machine_action(&mut self, action: &MachineAction) -> Option<TaskRequest> {
        match action {
            MachineAction::List => {
                self.execute_command(CommandSpec::new("docker-machine").args(["ls"]))
            }
            MachineAction::Start => self.execute_mutating_command(
                CommandSpec::new("docker-machine").args(["start", "default"]),
            ),
            MachineAction::Stop => self.execute_mutating_command(
                CommandSpec::new("docker-machine").args(["stop", "default"]),
            ),
            MachineAction::Env | MachineAction::Eval => {
                self.execute_command(CommandSpec::new("docker-machine").args(["env", "default"]))
            }
            MachineAction::Ip => {
                self.execute_command(CommandSpec::new("docker-machine").args(["ip", "default"]))
            }
        }
    }

    pub fn execute_selected(&mut self) -> Option<TaskRequest> {
        match self.current_tab {
            Tab::Dashboard => None,
            Tab::Container => {
                if self.container_table_focus {
                    let containers = self.filtered_containers();
                    if let Some(index) = self.container_table_state.selected()
                        && let Some(container) = containers.get(index)
                    {
                        return Some(TaskRequest::InspectContainer {
                            id: container.id.clone(),
                        });
                    }
                    return None;
                }
                if let Some(i) = self.container_list_state.selected()
                    && let Some(action) = self.container_actions.get(i).cloned()
                {
                    return self.run_container_action(&action);
                }
                None
            }
            Tab::Image => {
                if let Some(i) = self.image_list_state.selected()
                    && let Some(action) = self.image_actions.get(i).cloned()
                {
                    return self.run_image_action(&action);
                }
                None
            }
            Tab::Network => {
                if let Some(i) = self.network_list_state.selected()
                    && let Some(action) = self.network_actions.get(i).cloned()
                {
                    return self.run_network_action(&action);
                }
                None
            }
            Tab::Volume => {
                if let Some(i) = self.volume_list_state.selected()
                    && let Some(action) = self.volume_actions.get(i).cloned()
                {
                    return self.run_volume_action(&action);
                }
                None
            }
            Tab::Project => {
                if let Some(i) = self.project_list_state.selected()
                    && let Some(action) = self.project_actions.get(i).cloned()
                {
                    return self.run_project_action(&action);
                }
                None
            }
            Tab::Machine => {
                if let Some(i) = self.machine_list_state.selected()
                    && let Some(action) = self.machine_actions.get(i).cloned()
                {
                    return self.run_machine_action(&action);
                }
                None
            }
            Tab::Help => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_wraps_to_first_container_action() {
        let mut app = App::new();
        app.container_table_focus = false;
        app.container_list_state
            .select(Some(app.container_actions.len() - 1));

        app.next();

        assert_eq!(app.container_list_state.selected(), Some(0));
    }

    #[test]
    fn previous_wraps_to_last_container_action() {
        let mut app = App::new();
        app.container_table_focus = false;
        app.container_list_state.select(Some(0));

        app.previous();

        assert_eq!(
            app.container_list_state.selected(),
            Some(app.container_actions.len() - 1)
        );
    }

    #[test]
    fn empty_input_is_cancelled_without_running_an_action() {
        let mut app = App::new();
        app.start_input("prompt", PendingAction::ContainerLogs);

        app.confirm_input();

        assert!(!app.input_mode);
        assert!(app.pending_action.is_none());
        assert_eq!(
            app.output_lines.last().map(String::as_str),
            Some("[entrada cancelada: valor vacío]")
        );
    }

    #[test]
    fn container_filter_matches_name_and_sorts_by_selected_field() {
        let mut app = App::new();
        app.set_containers(vec![
            ContainerRow {
                id: "a".to_string(),
                name: "api".to_string(),
                image: "z-image".to_string(),
                state: "running".to_string(),
                status: "Up".to_string(),
            },
            ContainerRow {
                id: "b".to_string(),
                name: "worker".to_string(),
                image: "a-image".to_string(),
                state: "exited".to_string(),
                status: "Exited".to_string(),
            },
        ]);
        app.container_filter = "api".to_string();

        let containers = app.filtered_containers();

        assert_eq!(containers.len(), 1);
        assert_eq!(containers[0].name, "api");
    }

    #[test]
    fn selected_container_enters_inspect_request() {
        let mut app = App::new();
        app.set_containers(vec![ContainerRow {
            id: "container-id".to_string(),
            name: "api".to_string(),
            image: "image".to_string(),
            state: "running".to_string(),
            status: "Up".to_string(),
        }]);

        let request = app.execute_selected().expect("selected container request");

        assert!(matches!(
            request,
            TaskRequest::InspectContainer { id } if id == "container-id"
        ));
    }

    #[test]
    fn log_buffer_is_bounded_and_filterable() {
        let mut app = App::new();
        for index in 0..=MAX_LOG_LINES {
            app.push_log_line(format!("line {}", index));
        }
        app.log_filter = "line 2000".to_string();

        assert_eq!(app.log_lines.len(), MAX_LOG_LINES);
        assert_eq!(app.log_lines.front().map(String::as_str), Some("line 1"));
        assert_eq!(app.filtered_log_lines(), vec!["line 2000"]);
    }

    #[test]
    fn dashboard_request_is_read_only() {
        let app = App::new();
        let request = app.dashboard_request();

        assert_eq!(request.mutation(), Mutation::ReadOnly);
        assert!(matches!(
            request,
            TaskRequest::Dashboard { selected_id: None }
        ));
    }

    #[test]
    fn start_action_creates_named_container_request() {
        let mut app = App::new();
        app.container_table_focus = false;
        app.container_list_state.select(Some(1));
        app.execute_selected();
        app.input_buffer = "web".to_string();

        let request = app.confirm_input().expect("start request");

        assert!(matches!(
            request,
            TaskRequest::ContainerLifecycle {
                id,
                operation: ContainerLifecycle::Start
            } if id == "web"
        ));
    }

    #[test]
    fn stop_action_creates_named_container_request() {
        let mut app = App::new();
        app.container_table_focus = false;
        app.container_list_state.select(Some(3));
        app.execute_selected();
        app.input_buffer = "web".to_string();

        let request = app.confirm_input().expect("stop request");

        assert!(matches!(
            request,
            TaskRequest::ContainerLifecycle {
                id,
                operation: ContainerLifecycle::Stop
            } if id == "web"
        ));
    }

    #[test]
    fn compose_profile_action_creates_a_mutating_request() {
        let mut app = App::new();
        app.current_tab = Tab::Project;
        app.project_list_state.select(Some(2));
        app.execute_selected();
        app.input_buffer = "dev".to_string();

        let request = app.confirm_input().expect("compose profile request");

        assert_eq!(request.mutation(), Mutation::Mutating);
    }

    #[test]
    fn image_list_action_requests_bollard_image_listing() {
        let mut app = App::new();
        app.current_tab = Tab::Image;
        app.image_list_state.select(Some(2));

        let request = app.execute_selected().expect("image list request");

        assert!(matches!(request, TaskRequest::ListImages));
    }
}
