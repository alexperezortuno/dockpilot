use ratatui::widgets::ListState;

#[derive(Debug, Clone)]
pub enum ContainerAction {
    Start,
    StopAll,
    Stop,
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
    Container,
    Image,
    Network,
    Volume,
    Project,
    Machine,
    Help,
}

/// Actions that require a user-supplied parameter.
#[derive(Debug, Clone)]
pub enum PendingAction {
    // Container
    ContainerLogs,
    ContainerTop,
    ContainerDiff,
    ContainerPause,
    ContainerUnpause,
    ContainerUpdate,
    ContainerWait,
    ContainerRemove,
    ContainerCreate,
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
}

pub struct App {
    pub(crate) current_tab: Tab,
    pub(crate) container_actions: Vec<ContainerAction>,
    pub(crate) image_actions: Vec<ImageAction>,
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
    // Input mode
    pub(crate) input_mode: bool,
    pub(crate) input_buffer: String,
    pub(crate) input_prompt: String,
    pub(crate) pending_action: Option<PendingAction>,
}

impl App {
    pub fn new() -> Self {
        let mut app = App {
            current_tab: Tab::Container,
            container_actions: vec![
                ContainerAction::Start,
                ContainerAction::StopAll,
                ContainerAction::Stop,
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
            project_actions: vec![ProjectAction::SetFolder],
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
            project_folder: std::env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| ".".to_string()),
            input_mode: false,
            input_buffer: String::new(),
            input_prompt: String::new(),
            pending_action: None,
        };

        app.container_list_state.select(Some(0));
        app.image_list_state.select(Some(0));
        app.network_list_state.select(Some(0));
        app.volume_list_state.select(Some(0));
        app.project_list_state.select(Some(0));
        app.machine_list_state.select(Some(0));

        app
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

    pub fn next(&mut self) {
        match self.current_tab {
            Tab::Container => {
                Self::next_in_list(&mut self.container_list_state, self.container_actions.len())
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
            Tab::Container => {
                Self::previous_in_list(&mut self.container_list_state, self.container_actions.len())
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
            Tab::Container => Tab::Image,
            Tab::Image => Tab::Network,
            Tab::Network => Tab::Volume,
            Tab::Volume => Tab::Project,
            Tab::Project => Tab::Machine,
            Tab::Machine => Tab::Help,
            Tab::Help => Tab::Container,
        };
    }

    pub fn previous_tab(&mut self) {
        self.current_tab = match self.current_tab {
            Tab::Container => Tab::Help,
            Tab::Image => Tab::Container,
            Tab::Network => Tab::Image,
            Tab::Volume => Tab::Network,
            Tab::Project => Tab::Volume,
            Tab::Machine => Tab::Project,
            Tab::Help => Tab::Machine,
        };
    }

    // --- Input mode ---
    fn start_input(&mut self, prompt: &str, action: PendingAction) {
        self.input_mode = true;
        self.input_buffer.clear();
        self.input_prompt = prompt.to_string();
        self.pending_action = Some(action);
    }

    pub fn cancel_input(&mut self) {
        self.input_mode = false;
        self.input_buffer.clear();
        self.input_prompt.clear();
        self.pending_action = None;
    }

    pub fn confirm_input(&mut self) {
        let value = self.input_buffer.trim().to_string();
        let action = self.pending_action.take();
        self.input_mode = false;
        self.input_buffer.clear();
        self.input_prompt.clear();

        if value.is_empty() {
            self.output_lines
                .push("[entrada cancelada: valor vacío]".to_string());
            return;
        }

        if let Some(action) = action {
            self.execute_pending_action(action, &value);
        }
    }

    fn execute_pending_action(&mut self, action: PendingAction, value: &str) {
        match action {
            // Container
            PendingAction::ContainerLogs => {
                self.execute_command(&format!("docker logs --tail 100 {}", value));
            }
            PendingAction::ContainerTop => {
                self.execute_command(&format!("docker container top {}", value));
            }
            PendingAction::ContainerDiff => {
                self.execute_command(&format!("docker container diff {}", value));
            }
            PendingAction::ContainerPause => {
                self.execute_command(&format!("docker container pause {}", value));
            }
            PendingAction::ContainerUnpause => {
                self.execute_command(&format!("docker container unpause {}", value));
            }
            PendingAction::ContainerUpdate => {
                self.execute_command(&format!("docker container update --memory=512m {}", value));
            }
            PendingAction::ContainerWait => {
                self.execute_command(&format!("docker container wait {}", value));
            }
            PendingAction::ContainerRemove => {
                self.execute_command(&format!("docker rm -f {}", value));
            }
            PendingAction::ContainerCreate => {
                // value = "image name" e.g. "test nginx"
                let mut parts = value.splitn(2, ' ');
                let name = parts.next().unwrap_or("test");
                let image = parts.next().unwrap_or("nginx");
                self.execute_command(&format!("docker run -d --name={} -p 83:83 {}", name, image));
            }
            // Image
            PendingAction::ImageRemove => {
                self.execute_command(&format!("docker rmi {}", value));
            }
            PendingAction::ImagePush => {
                self.execute_command(&format!("docker push {}", value));
            }
            PendingAction::ImagePull => {
                self.execute_command(&format!("docker pull {}", value));
            }
            PendingAction::ImageSave => {
                self.execute_command(&format!("docker save {} > image.tar", value));
            }
            PendingAction::ImageLoad => {
                // value = path to the tar file, defaults to image.tar
                self.execute_command(&format!("docker load < {}", value));
            }
            PendingAction::ImageHistory => {
                self.execute_command(&format!("docker history {}", value));
            }
            PendingAction::ImageBuild => {
                // value = "tag context"  p.ej. "myimage ."
                let mut parts = value.splitn(2, ' ');
                let tag = parts.next().unwrap_or("myimage");
                let ctx = parts.next().unwrap_or(".");
                self.execute_command(&format!("docker build -t {} {}", tag, ctx));
            }
            PendingAction::ImageRebuild => {
                let mut parts = value.splitn(2, ' ');
                let tag = parts.next().unwrap_or("myimage");
                let ctx = parts.next().unwrap_or(".");
                self.execute_command(&format!("docker build --no-cache -t {} {}", tag, ctx));
            }
            // Network
            PendingAction::NetworkCreate => {
                self.execute_command(&format!("docker network create {}", value));
            }
            PendingAction::NetworkRemove => {
                self.execute_command(&format!("docker network rm {}", value));
            }
            // Volume
            PendingAction::VolumeCreate => {
                self.execute_command(&format!("docker volume create {}", value));
            }
            PendingAction::VolumeRemove => {
                self.execute_command(&format!("docker volume rm {}", value));
            }
            // Project
            PendingAction::ProjectSetFolder => {
                self.project_folder = value.to_string();
                let folder = self.project_folder.clone();
                self.output_lines
                    .push(format!("[proyecto] carpeta establecida: {}", folder));
                self.execute_command(&format!("ls -la {}", folder));
            }
        }
    }

    // --- Command executions ---
    fn execute_command(&mut self, command: &str) {
        crate::docker::execute_command(&mut self.output_lines, &mut self.output_scroll, command);
    }

    // --- Tab actions ---
    fn run_container_action(&mut self, action: &ContainerAction) {
        match action {
            ContainerAction::Start => self.execute_command("docker compose up -d"),
            ContainerAction::StopAll => self.execute_command("docker stop $(docker ps -aq)"),
            ContainerAction::Stop => self.execute_command("docker compose stop"),
            ContainerAction::ListAll => self.execute_command("docker ps -a"),
            ContainerAction::List => self.execute_command("docker ps"),
            ContainerAction::Logs => {
                self.start_input("Contenedor para ver logs:", PendingAction::ContainerLogs);
            }
            ContainerAction::Create => {
                self.start_input(
                    "Nombre e imagen (ej: test nginx):",
                    PendingAction::ContainerCreate,
                );
            }
            ContainerAction::Remove => {
                self.start_input(
                    "Contenedor a eliminar (ID o nombre):",
                    PendingAction::ContainerRemove,
                );
            }
            ContainerAction::Top => {
                self.start_input("Contenedor para top:", PendingAction::ContainerTop);
            }
            ContainerAction::Diff => {
                self.start_input("Contenedor para diff:", PendingAction::ContainerDiff);
            }
            ContainerAction::Pause => {
                self.start_input("Contenedor a pausar:", PendingAction::ContainerPause);
            }
            ContainerAction::Unpause => {
                self.start_input("Contenedor a reanudar:", PendingAction::ContainerUnpause);
            }
            ContainerAction::Update => {
                self.start_input("Contenedor a actualizar:", PendingAction::ContainerUpdate);
            }
            ContainerAction::Wait => {
                self.start_input("Contenedor a esperar:", PendingAction::ContainerWait);
            }
        }
    }

    fn run_image_action(&mut self, action: &ImageAction) {
        match action {
            ImageAction::Build => {
                self.start_input("Tag y contexto (ej: myimage .):", PendingAction::ImageBuild);
            }
            ImageAction::Rebuild => {
                self.start_input(
                    "Tag y contexto (ej: myimage .):",
                    PendingAction::ImageRebuild,
                );
            }
            ImageAction::List => self.execute_command("docker image list"),
            ImageAction::Remove => {
                self.start_input(
                    "Imagen a eliminar (ID o nombre):",
                    PendingAction::ImageRemove,
                );
            }
            ImageAction::Push => {
                self.start_input("Imagen a subir (tag):", PendingAction::ImagePush);
            }
            ImageAction::Pull => {
                self.start_input("Imagen a descargar:", PendingAction::ImagePull);
            }
            ImageAction::Save => {
                self.start_input("Imagen a guardar:", PendingAction::ImageSave);
            }
            ImageAction::Load => {
                self.start_input("Ruta del tar (ej: image.tar):", PendingAction::ImageLoad);
            }
            ImageAction::History => {
                self.start_input("Imagen a inspeccionar:", PendingAction::ImageHistory);
            }
        }
    }

    fn run_network_action(&mut self, action: &NetworkAction) {
        match action {
            NetworkAction::List => self.execute_command("docker network ls"),
            NetworkAction::Create => {
                self.start_input("Nombre de la nueva red:", PendingAction::NetworkCreate);
            }
            NetworkAction::Remove => {
                self.start_input("Nombre de la red a eliminar:", PendingAction::NetworkRemove);
            }
        }
    }

    fn run_volume_action(&mut self, action: &VolumeAction) {
        match action {
            VolumeAction::List => self.execute_command("docker volume ls"),
            VolumeAction::Create => {
                self.start_input("Nombre del nuevo volumen:", PendingAction::VolumeCreate);
            }
            VolumeAction::Remove => {
                self.start_input(
                    "Nombre del volumen a eliminar:",
                    PendingAction::VolumeRemove,
                );
            }
        }
    }

    fn run_project_action(&mut self, action: &ProjectAction) {
        match action {
            ProjectAction::SetFolder => {
                self.start_input(
                    "Ruta de la carpeta del proyecto:",
                    PendingAction::ProjectSetFolder,
                );
            }
        }
    }

    fn run_machine_action(&mut self, action: &MachineAction) {
        match action {
            MachineAction::List => self.execute_command("docker-machine ls"),
            MachineAction::Start => self.execute_command("docker-machine start default"),
            MachineAction::Stop => self.execute_command("docker-machine stop default"),
            MachineAction::Env => self.execute_command("docker-machine env default"),
            MachineAction::Eval => self.execute_command("eval $(docker-machine env default)"),
            MachineAction::Ip => self.execute_command("docker-machine ip default"),
        }
    }

    pub fn execute_selected(&mut self) {
        match self.current_tab {
            Tab::Container => {
                if let Some(i) = self.container_list_state.selected()
                    && let Some(action) = self.container_actions.get(i).cloned()
                {
                    self.run_container_action(&action);
                }
            }
            Tab::Image => {
                if let Some(i) = self.image_list_state.selected()
                    && let Some(action) = self.image_actions.get(i).cloned()
                {
                    self.run_image_action(&action);
                }
            }
            Tab::Network => {
                if let Some(i) = self.network_list_state.selected()
                    && let Some(action) = self.network_actions.get(i).cloned()
                {
                    self.run_network_action(&action);
                }
            }
            Tab::Volume => {
                if let Some(i) = self.volume_list_state.selected()
                    && let Some(action) = self.volume_actions.get(i).cloned()
                {
                    self.run_volume_action(&action);
                }
            }
            Tab::Project => {
                if let Some(i) = self.project_list_state.selected()
                    && let Some(action) = self.project_actions.get(i).cloned()
                {
                    self.run_project_action(&action);
                }
            }
            Tab::Machine => {
                if let Some(i) = self.machine_list_state.selected()
                    && let Some(action) = self.machine_actions.get(i).cloned()
                {
                    self.run_machine_action(&action);
                }
            }
            Tab::Help => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_wraps_to_first_container_action() {
        let mut app = App::new();
        app.container_list_state
            .select(Some(app.container_actions.len() - 1));

        app.next();

        assert_eq!(app.container_list_state.selected(), Some(0));
    }

    #[test]
    fn previous_wraps_to_last_container_action() {
        let mut app = App::new();
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
}
