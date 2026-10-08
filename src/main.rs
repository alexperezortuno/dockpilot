use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Tabs},
    Frame, Terminal,
};
use std::io;
use std::process::Command;

#[derive(Debug, Clone)]
enum ContainerAction {
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
enum ImageAction {
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
enum NetworkAction {
    List,
    Create,
    Remove,
}

#[derive(Debug, Clone)]
enum VolumeAction {
    List,
    Create,
    Remove,
}

#[derive(Debug, Clone)]
enum ProjectAction {
    SetFolder,
}

#[derive(Debug, Clone)]
enum MachineAction {
    List,
    Start,
    Stop,
    Env,
    Eval,
    Ip,
}

#[derive(Debug, Clone, PartialEq)]
enum Tab {
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
enum PendingAction {
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

struct App {
    current_tab: Tab,
    container_actions: Vec<ContainerAction>,
    image_actions: Vec<ImageAction>,
    network_actions: Vec<NetworkAction>,
    volume_actions: Vec<VolumeAction>,
    project_actions: Vec<ProjectAction>,
    machine_actions: Vec<MachineAction>,
    output_lines: Vec<String>,
    container_list_state: ListState,
    image_list_state: ListState,
    network_list_state: ListState,
    volume_list_state: ListState,
    project_list_state: ListState,
    machine_list_state: ListState,
    output_scroll: u16,
    project_folder: String,
    // Input mode
    input_mode: bool,
    input_buffer: String,
    input_prompt: String,
    pending_action: Option<PendingAction>,
}

impl App {
    fn new() -> Self {
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

    fn next(&mut self) {
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

    fn previous(&mut self) {
        match self.current_tab {
            Tab::Container => Self::previous_in_list(
                &mut self.container_list_state,
                self.container_actions.len(),
            ),
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

    fn next_tab(&mut self) {
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

    fn previous_tab(&mut self) {
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

    fn cancel_input(&mut self) {
        self.input_mode = false;
        self.input_buffer.clear();
        self.input_prompt.clear();
        self.pending_action = None;
    }

    fn confirm_input(&mut self) {
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
                self.execute_command(&format!(
                    "docker run -d --name={} -p 83:83 {}",
                    name, image
                ));
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
        self.output_lines.push(format!("$ {}", command));
        self.output_scroll = self.output_lines.len() as u16;

        let output = Command::new("sh").arg("-c").arg(command).output();

        match output {
            Ok(out) => {
                if !out.stdout.is_empty() {
                    let stdout = String::from_utf8_lossy(&out.stdout);
                    for line in stdout.lines() {
                        self.output_lines.push(line.to_string());
                    }
                }
                if !out.stderr.is_empty() {
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    for line in stderr.lines() {
                        self.output_lines.push(format!("[stderr] {}", line));
                    }
                }
                if !out.status.success() {
                    self.output_lines
                        .push(format!("[exit code: {:?}]", out.status.code()));
                }
            }
            Err(e) => {
                self.output_lines
                    .push(format!("[error ejecutando comando: {}]", e));
            }
        }

        self.output_lines.push(String::new());
        self.output_scroll = self.output_lines.len() as u16;
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
                self.start_input(
                    "Tag y contexto (ej: myimage .):",
                    PendingAction::ImageBuild,
                );
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
                self.start_input(
                    "Ruta del tar (ej: image.tar):",
                    PendingAction::ImageLoad,
                );
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
                self.start_input(
                    "Nombre de la nueva red:",
                    PendingAction::NetworkCreate,
                );
            }
            NetworkAction::Remove => {
                self.start_input(
                    "Nombre de la red a eliminar:",
                    PendingAction::NetworkRemove,
                );
            }
        }
    }

    fn run_volume_action(&mut self, action: &VolumeAction) {
        match action {
            VolumeAction::List => self.execute_command("docker volume ls"),
            VolumeAction::Create => {
                self.start_input(
                    "Nombre del nuevo volumen:",
                    PendingAction::VolumeCreate,
                );
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

    fn execute_selected(&mut self) {
        match self.current_tab {
            Tab::Container => {
                if let Some(i) = self.container_list_state.selected() {
                    if let Some(action) = self.container_actions.get(i).cloned() {
                        self.run_container_action(&action);
                    }
                }
            }
            Tab::Image => {
                if let Some(i) = self.image_list_state.selected() {
                    if let Some(action) = self.image_actions.get(i).cloned() {
                        self.run_image_action(&action);
                    }
                }
            }
            Tab::Network => {
                if let Some(i) = self.network_list_state.selected() {
                    if let Some(action) = self.network_actions.get(i).cloned() {
                        self.run_network_action(&action);
                    }
                }
            }
            Tab::Volume => {
                if let Some(i) = self.volume_list_state.selected() {
                    if let Some(action) = self.volume_actions.get(i).cloned() {
                        self.run_volume_action(&action);
                    }
                }
            }
            Tab::Project => {
                if let Some(i) = self.project_list_state.selected() {
                    if let Some(action) = self.project_actions.get(i).cloned() {
                        self.run_project_action(&action);
                    }
                }
            }
            Tab::Machine => {
                if let Some(i) = self.machine_list_state.selected() {
                    if let Some(action) = self.machine_actions.get(i).cloned() {
                        self.run_machine_action(&action);
                    }
                }
            }
            Tab::Help => {}
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    let mut should_quit = false;

    while !should_quit {
        terminal.draw(|f| draw_app(f, &mut app))?;

        if let Event::Key(KeyEvent { code, modifiers, .. }) = event::read()? {
            if app.input_mode {
                // Input mode: characters go to the buffer
                match code {
                    KeyCode::Enter => app.confirm_input(),
                    KeyCode::Esc => app.cancel_input(),
                    KeyCode::Backspace => {
                        app.input_buffer.pop();
                    }
                    KeyCode::Char(c) => {
                        app.input_buffer.push(c);
                    }
                    _ => {}
                }
            } else {
                // Normal mode
                match (code, modifiers) {
                    (KeyCode::Char('q'), _) | (KeyCode::Esc, _) => should_quit = true,
                    (KeyCode::Char('c'), KeyModifiers::CONTROL) => should_quit = true,
                    (KeyCode::Tab, _) => app.next_tab(),
                    (KeyCode::BackTab, _) => app.previous_tab(),
                    (KeyCode::Up, _) => app.previous(),
                    (KeyCode::Down, _) => app.next(),
                    (KeyCode::Enter, _) => app.execute_selected(),
                    _ => {}
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

fn draw_app(f: &mut Frame, app: &mut App) {
    let size = f.size();

    // If we are in input mode, we reserve 3 lines at the bottom for the prompt.
    let (main_area, input_area) = if app.input_mode {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(10), Constraint::Length(3)])
            .split(size);
        (chunks[0], Some(chunks[1]))
    } else {
        (size, None)
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(15),
            Constraint::Min(10),
        ])
        .split(main_area);

    // Tabs
    let tab_titles: Vec<Span> = vec![
        "Container",
        "Image",
        "Network",
        "Volume",
        "Project",
        "Machine",
        "Help",
    ]
        .iter()
        .map(|t| Span::raw(*t))
        .collect();

    let selected_tab = match app.current_tab {
        Tab::Container => 0,
        Tab::Image => 1,
        Tab::Network => 2,
        Tab::Volume => 3,
        Tab::Project => 4,
        Tab::Machine => 5,
        Tab::Help => 6,
    };

    let tabs = Tabs::new(tab_titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Docker Simplifier "),
        )
        .select(selected_tab)
        .style(Style::default().fg(Color::White))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );

    f.render_widget(tabs, chunks[0]);

    // List of actions
    let (items, title, state): (Vec<ListItem>, &str, &mut ListState) = match app.current_tab {
        Tab::Container => {
            let items = app
                .container_actions
                .iter()
                .map(|a| {
                    let s = match a {
                        ContainerAction::Start => "Start Containers",
                        ContainerAction::StopAll => "Stop all Containers",
                        ContainerAction::Stop => "Stop Containers",
                        ContainerAction::ListAll => "List All Containers",
                        ContainerAction::List => "List Containers",
                        ContainerAction::Logs => "View Container Logs",
                        ContainerAction::Create => "Create Container",
                        ContainerAction::Remove => "Remove Container",
                        ContainerAction::Top => "View Container Top",
                        ContainerAction::Diff => "View Container Diff",
                        ContainerAction::Pause => "Pause Container",
                        ContainerAction::Unpause => "Unpause Container",
                        ContainerAction::Update => "Update Container",
                        ContainerAction::Wait => "Wait for Container",
                    };
                    ListItem::new(s)
                })
                .collect();
            (items, "Container Actions", &mut app.container_list_state)
        }
        Tab::Image => {
            let items = app
                .image_actions
                .iter()
                .map(|a| {
                    let s = match a {
                        ImageAction::Build => "Build Image",
                        ImageAction::Rebuild => "Rebuild Image (no-cache)",
                        ImageAction::List => "List Images",
                        ImageAction::Remove => "Remove Image",
                        ImageAction::Push => "Push Image",
                        ImageAction::Pull => "Pull Image",
                        ImageAction::Save => "Save Image to tar",
                        ImageAction::Load => "Load Image from tar",
                        ImageAction::History => "View Image History",
                    };
                    ListItem::new(s)
                })
                .collect();
            (items, "Image Actions", &mut app.image_list_state)
        }
        Tab::Network => {
            let items = app
                .network_actions
                .iter()
                .map(|a| {
                    let s = match a {
                        NetworkAction::List => "List Networks",
                        NetworkAction::Create => "Create Network",
                        NetworkAction::Remove => "Remove Network",
                    };
                    ListItem::new(s)
                })
                .collect();
            (items, "Network Actions", &mut app.network_list_state)
        }
        Tab::Volume => {
            let items = app
                .volume_actions
                .iter()
                .map(|a| {
                    let s = match a {
                        VolumeAction::List => "List Volumes",
                        VolumeAction::Create => "Create Volume",
                        VolumeAction::Remove => "Remove Volume",
                    };
                    ListItem::new(s)
                })
                .collect();
            (items, "Volume Actions", &mut app.volume_list_state)
        }
        Tab::Project => {
            let items = app
                .project_actions
                .iter()
                .map(|a| {
                    let s = match a {
                        ProjectAction::SetFolder => "Set Project Folder",
                    };
                    ListItem::new(s)
                })
                .collect();
            (items, "Project Actions", &mut app.project_list_state)
        }
        Tab::Machine => {
            let items = app
                .machine_actions
                .iter()
                .map(|a| {
                    let s = match a {
                        MachineAction::List => "List Machines",
                        MachineAction::Start => "Start Machine",
                        MachineAction::Stop => "Stop Machine",
                        MachineAction::Env => "Show Machine Env",
                        MachineAction::Eval => "Eval Machine Env",
                        MachineAction::Ip => "Get Machine IP",
                    };
                    ListItem::new(s)
                })
                .collect();
            (items, "Machine Actions", &mut app.machine_list_state)
        }
        Tab::Help => {
            let help = vec![
                "Docker Simplifier TUI",
                "",
                "Controls:",
                "  Tab / Shift+Tab - Switch tab",
                "  Up/Down         - Navigate actions",
                "  Enter           - Execute / prompt for parameter",
                "  q / Esc         - Exit",
                "",
                "Input mode:",
                "  Enter    - Confirm",
                "  Esc      - Cancel",
                "  Backspace- Delete",
                "",
                "Available tabs:",
                "  Container, Image, Network, Volume, Project, Machine",
            ];
            let items: Vec<ListItem> = help.iter().map(|l| ListItem::new(*l)).collect();
            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL).title("Help"));
            f.render_widget(list, chunks[1]);
            draw_output(f, app, chunks[2]);
            if let Some(area) = input_area {
                draw_input(f, app, area);
            }
            return;
        }
    };

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(
            Style::default()
                .bg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    f.render_stateful_widget(list, chunks[1], state);

    draw_output(f, app, chunks[2]);

    if let Some(area) = input_area {
        draw_input(f, app, area);
    }
}

fn draw_output(f: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
    let visible_height = area.height.saturating_sub(2) as usize;
    let total = app.output_lines.len();
    let start = if total > visible_height {
        total - visible_height
    } else {
        0
    };

    let items: Vec<ListItem> = app
        .output_lines
        .iter()
        .skip(start)
        .map(|l| ListItem::new(l.clone()))
        .collect();

    let output_list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" Output ({} líneas) ", total)),
    );

    f.render_widget(output_list, area);
}

fn draw_input(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let text = format!("{} {}", app.input_prompt, app.input_buffer);
    let input_widget = Paragraph::new(text).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Entrada (Enter=OK, Esc=Cancelar) ")
            .border_style(Style::default().fg(Color::Yellow)),
    );
    f.render_widget(input_widget, area);
}