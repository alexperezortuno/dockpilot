use crate::{
    config::ThemeName,
    docker::{
        CommandSpec,
        client::{
            ContainerLifecycle, ContainerRow, DashboardData, HealthSnapshot, ImageRow, NetworkRow,
            VolumeRow,
        },
    },
    security::{Mutation, SafetyPolicy},
    tasks::TaskRequest,
};
use ratatui::widgets::ListState;
use std::{collections::VecDeque, path::PathBuf};

#[allow(dead_code)]
pub mod alerts;
pub mod commands;
#[allow(dead_code)]
pub mod health;
#[allow(dead_code)]
pub mod logs;
#[allow(dead_code)]
pub mod monitoring;
mod navigation;
pub mod notifications;
mod output;
pub mod search;
#[allow(dead_code)]
pub mod task_history;

const MAX_LOG_LINES: usize = 2_000;
const MAX_EVENT_LINES: usize = 500;
const MAX_ALERTS: usize = 100;

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
    Backup,
    Restore,
}

#[derive(Debug, Clone)]
pub enum ProjectAction {
    SetFolder,
    ComposeUp,
    ComposeUpProfile,
    ComposeDown,
    ComposeConfig,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceKind {
    Container,
    Image,
    Network,
    Volume,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextAction {
    Inspect,
    Start,
    Stop,
    Restart,
    Logs,
    Remove,
    History,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Tab {
    Dashboard,
    Container,
    Image,
    Network,
    Volume,
    Project,
    Help,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusTarget {
    Table,
    Actions,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Overlay {
    None,
    Palette { query: String, selected: usize },
    Search(search::SearchState),
    Context { selected: usize },
}

impl FocusTarget {
    fn next(self) -> Self {
        match self {
            Self::Table => Self::Actions,
            Self::Actions => Self::Table,
        }
    }

    fn previous(self) -> Self {
        self.next()
    }
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
    ImageFilter,
    NetworkFilter,
    VolumeFilter,
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
    VolumeBackup,
    VolumeRestore,
    // Project
    ProjectSetFolder,
    ComposeProfileUp,
}

pub struct App {
    pub(crate) current_tab: Tab,
    pub(crate) focus_target: FocusTarget,
    resource_focus: [FocusTarget; 4],
    pub(crate) container_actions: Vec<ContainerAction>,
    pub(crate) image_actions: Vec<ImageAction>,
    pub(crate) images: Vec<ImageRow>,
    pub(crate) image_filter: String,
    pub(crate) image_table_state: ratatui::widgets::TableState,
    pub(crate) network_actions: Vec<NetworkAction>,
    pub(crate) networks: Vec<NetworkRow>,
    pub(crate) network_filter: String,
    pub(crate) network_table_state: ratatui::widgets::TableState,
    pub(crate) volume_actions: Vec<VolumeAction>,
    pub(crate) volumes: Vec<VolumeRow>,
    pub(crate) volume_filter: String,
    pub(crate) volume_table_state: ratatui::widgets::TableState,
    pub(crate) project_actions: Vec<ProjectAction>,
    pub(crate) output_lines: VecDeque<String>,
    pub(crate) output_capacity: usize,
    pub(crate) container_list_state: ListState,
    pub(crate) image_list_state: ListState,
    pub(crate) network_list_state: ListState,
    pub(crate) volume_list_state: ListState,
    pub(crate) project_list_state: ListState,
    pub(crate) output_scroll: usize,
    pub(crate) project_folder: String,
    pub(crate) engine_status: String,
    pub(crate) task_status: String,
    pub(crate) theme: ThemeName,
    pub(crate) dashboard: Option<DashboardData>,
    pub(crate) log_lines: VecDeque<String>,
    pub(crate) log_filter: String,
    pub(crate) log_level_filter: Option<logs::LogLevel>,
    pub(crate) log_timestamps: bool,
    pub(crate) logs_paused: bool,
    pub(crate) event_lines: VecDeque<String>,
    pub(crate) alerts: VecDeque<String>,
    pub(crate) containers: Vec<ContainerRow>,
    pub(crate) container_table_state: ratatui::widgets::TableState,
    pub(crate) container_filter: String,
    pub(crate) container_sort: ContainerSort,
    // Input mode
    pub(crate) input_mode: bool,
    pub(crate) input_buffer: String,
    pub(crate) input_prompt: String,
    pub(crate) pending_action: Option<PendingAction>,
    pub(crate) overlay: Overlay,
    pub(crate) notifications: notifications::NotificationQueue,
    pub(crate) policy: SafetyPolicy,
    pub(crate) task_history: task_history::TaskHistory,
    pub(crate) metrics_history: monitoring::MetricsHistory,
    pub(crate) latest_metrics: Option<monitoring::MetricSample>,
    pub(crate) health: Option<HealthSnapshot>,
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
            focus_target: FocusTarget::Table,
            resource_focus: [
                FocusTarget::Table,
                FocusTarget::Actions,
                FocusTarget::Actions,
                FocusTarget::Actions,
            ],
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
            image_filter: String::new(),
            image_table_state: ratatui::widgets::TableState::default(),
            network_actions: vec![
                NetworkAction::List,
                NetworkAction::Create,
                NetworkAction::Remove,
            ],
            networks: Vec::new(),
            network_filter: String::new(),
            network_table_state: ratatui::widgets::TableState::default(),
            volume_actions: vec![
                VolumeAction::List,
                VolumeAction::Create,
                VolumeAction::Remove,
                VolumeAction::Backup,
                VolumeAction::Restore,
            ],
            volumes: Vec::new(),
            volume_filter: String::new(),
            volume_table_state: ratatui::widgets::TableState::default(),
            project_actions: vec![
                ProjectAction::SetFolder,
                ProjectAction::ComposeUp,
                ProjectAction::ComposeUpProfile,
                ProjectAction::ComposeDown,
                ProjectAction::ComposeConfig,
            ],
            output_lines: VecDeque::new(),
            output_capacity: 2_000,
            container_list_state: ListState::default(),
            image_list_state: ListState::default(),
            network_list_state: ListState::default(),
            volume_list_state: ListState::default(),
            project_list_state: ListState::default(),
            output_scroll: 0,
            project_folder,
            engine_status: "checking Docker Engine".to_string(),
            task_status: "idle".to_string(),
            theme: ThemeName::Dark,
            dashboard: None,
            log_lines: VecDeque::new(),
            log_filter: String::new(),
            log_level_filter: None,
            log_timestamps: false,
            logs_paused: false,
            event_lines: VecDeque::new(),
            alerts: VecDeque::new(),
            containers: Vec::new(),
            container_table_state: ratatui::widgets::TableState::default(),
            container_filter: String::new(),
            container_sort: ContainerSort::Name,
            input_mode: false,
            input_buffer: String::new(),
            input_prompt: String::new(),
            pending_action: None,
            overlay: Overlay::None,
            notifications: notifications::NotificationQueue::default(),
            policy: SafetyPolicy::new(false, false),
            task_history: task_history::TaskHistory::new(100),
            metrics_history: monitoring::MetricsHistory::new(120),
            latest_metrics: None,
            health: None,
        };

        app.container_list_state.select(Some(0));
        app.image_list_state.select(Some(0));
        app.network_list_state.select(Some(0));
        app.volume_list_state.select(Some(0));
        app.project_list_state.select(Some(0));

        app
    }

    pub fn push_output(&mut self, line: impl Into<String>) {
        output::push(&mut self.output_lines, self.output_capacity, line);
        self.output_scroll = self.output_lines.len();
    }

    pub fn append_output(&mut self, lines: impl IntoIterator<Item = String>) {
        for line in lines {
            self.push_output(line);
        }
    }

    pub fn set_output_capacity(&mut self, capacity: usize) {
        self.output_capacity = capacity.max(1);
        output::resize(&mut self.output_lines, self.output_capacity);
        self.output_scroll = self.output_lines.len();
    }

    pub fn clear_output(&mut self) {
        self.output_lines.clear();
        self.output_scroll = 0;
    }

    pub fn scroll_output_up(&mut self, amount: usize) {
        self.output_scroll = self.output_scroll.saturating_sub(amount);
    }

    pub fn scroll_output_down(&mut self, amount: usize) {
        self.output_scroll = self.output_scroll.saturating_add(amount);
    }

    pub fn scroll_output_home(&mut self) {
        self.output_scroll = 0;
    }

    pub fn scroll_output_end(&mut self) {
        self.output_scroll = self.output_lines.len();
    }

    pub fn set_containers(&mut self, containers: Vec<ContainerRow>) {
        let selected = self.selected_resource_id(ResourceKind::Container);
        self.containers = containers;
        self.restore_selection(&selected, ResourceKind::Container);
    }

    pub fn set_images(&mut self, images: Vec<ImageRow>) {
        let selected = self.selected_resource_id(ResourceKind::Image);
        self.images = images;
        self.restore_selection(&selected, ResourceKind::Image);
    }

    pub fn set_networks(&mut self, networks: Vec<NetworkRow>) {
        let selected = self.selected_resource_id(ResourceKind::Network);
        self.networks = networks;
        self.restore_selection(&selected, ResourceKind::Network);
    }

    pub fn set_volumes(&mut self, volumes: Vec<VolumeRow>) {
        let selected = self.selected_resource_id(ResourceKind::Volume);
        self.volumes = volumes;
        self.restore_selection(&selected, ResourceKind::Volume);
    }

    fn selected_resource_id(&self, kind: ResourceKind) -> Option<String> {
        let index = match kind {
            ResourceKind::Container => self.container_table_state.selected(),
            ResourceKind::Image => self.image_table_state.selected(),
            ResourceKind::Network => self.network_table_state.selected(),
            ResourceKind::Volume => self.volume_table_state.selected(),
        }?;
        match kind {
            ResourceKind::Container => self.filtered_containers().get(index).map(|r| r.id.clone()),
            ResourceKind::Image => self.filtered_images().get(index).map(|r| r.id.clone()),
            ResourceKind::Network => self.filtered_networks().get(index).map(|r| r.id.clone()),
            ResourceKind::Volume => self.filtered_volumes().get(index).map(|r| r.name.clone()),
        }
    }

    fn restore_selection(&mut self, selected: &Option<String>, kind: ResourceKind) {
        let index = selected.as_ref().and_then(|id| match kind {
            ResourceKind::Container => self.filtered_containers().iter().position(|r| &r.id == id),
            ResourceKind::Image => self.filtered_images().iter().position(|r| &r.id == id),
            ResourceKind::Network => self.filtered_networks().iter().position(|r| &r.id == id),
            ResourceKind::Volume => self.filtered_volumes().iter().position(|r| &r.name == id),
        });
        let has_rows = match kind {
            ResourceKind::Container => !self.filtered_containers().is_empty(),
            ResourceKind::Image => !self.filtered_images().is_empty(),
            ResourceKind::Network => !self.filtered_networks().is_empty(),
            ResourceKind::Volume => !self.filtered_volumes().is_empty(),
        };
        let index = index.or(has_rows.then_some(0));
        match kind {
            ResourceKind::Container => self.container_table_state.select(index),
            ResourceKind::Image => self.image_table_state.select(index),
            ResourceKind::Network => self.network_table_state.select(index),
            ResourceKind::Volume => self.volume_table_state.select(index),
        }
    }

    pub fn toggle_focus(&mut self) {
        self.focus_next();
    }

    fn resource_focus_index(tab: &Tab) -> Option<usize> {
        match tab {
            Tab::Container => Some(0),
            Tab::Image => Some(1),
            Tab::Network => Some(2),
            Tab::Volume => Some(3),
            _ => None,
        }
    }

    fn sync_focus_target(&mut self) {
        self.focus_target = Self::resource_focus_index(&self.current_tab)
            .map(|index| self.resource_focus[index])
            .unwrap_or(FocusTarget::Actions);
    }

    fn set_focus_target(&mut self, target: FocusTarget) {
        self.focus_target = target;
        if let Some(index) = Self::resource_focus_index(&self.current_tab) {
            self.resource_focus[index] = target;
        }
    }

    pub fn focus_next(&mut self) {
        if Self::resource_focus_index(&self.current_tab).is_some() {
            self.set_focus_target(self.focus_target.next());
        }
    }

    pub fn focus_previous(&mut self) {
        if Self::resource_focus_index(&self.current_tab).is_some() {
            self.set_focus_target(self.focus_target.previous());
        }
    }

    pub fn toggle_container_sort(&mut self) {
        self.container_sort = match self.container_sort {
            ContainerSort::Name => ContainerSort::Image,
            ContainerSort::Image => ContainerSort::State,
            ContainerSort::State => ContainerSort::Name,
        };
        self.container_table_state.select(Some(0));
    }

    #[cfg(test)]
    pub fn start_container_filter(&mut self) {
        self.start_input("Filtro de contenedores:", PendingAction::ContainerFilter);
    }

    pub fn start_filter(&mut self) {
        match self.current_tab {
            Tab::Container => {
                self.start_input("Filtro de contenedores:", PendingAction::ContainerFilter);
            }
            Tab::Image => {
                self.start_input("Filtro de imagenes:", PendingAction::ImageFilter);
            }
            Tab::Network => {
                self.start_input("Filtro de redes:", PendingAction::NetworkFilter);
            }
            Tab::Volume => {
                self.start_input("Filtro de volumenes:", PendingAction::VolumeFilter);
            }
            _ => {}
        }
    }

    pub fn start_search(&mut self) {
        if let Some(filter) = self.current_filter() {
            self.overlay = Overlay::Search(search::SearchState::new(filter));
        }
    }

    pub fn search_push(&mut self, character: char) {
        if let Overlay::Search(state) = &mut self.overlay {
            state.query.push(character);
            let query = state.query.clone();
            self.set_current_filter(query);
        }
    }

    pub fn search_backspace(&mut self) {
        if let Overlay::Search(state) = &mut self.overlay {
            state.query.pop();
            let query = state.query.clone();
            self.set_current_filter(query);
        }
    }

    pub fn finish_search(&mut self) {
        if matches!(self.overlay, Overlay::Search(_)) {
            self.overlay = Overlay::None;
        }
    }

    pub fn cancel_search(&mut self) {
        if let Overlay::Search(state) = &self.overlay {
            self.set_current_filter(state.previous_filter.clone());
        }
        self.overlay = Overlay::None;
    }

    fn current_filter(&self) -> Option<String> {
        match self.current_tab {
            Tab::Container => Some(self.container_filter.clone()),
            Tab::Image => Some(self.image_filter.clone()),
            Tab::Network => Some(self.network_filter.clone()),
            Tab::Volume => Some(self.volume_filter.clone()),
            _ => None,
        }
    }

    fn set_current_filter(&mut self, filter: String) {
        match self.current_tab {
            Tab::Container => {
                let selected = self.selected_resource_id(ResourceKind::Container);
                self.container_filter = filter;
                self.restore_selection(&selected, ResourceKind::Container);
            }
            Tab::Image => {
                let selected = self.selected_resource_id(ResourceKind::Image);
                self.image_filter = filter;
                self.restore_selection(&selected, ResourceKind::Image);
            }
            Tab::Network => {
                let selected = self.selected_resource_id(ResourceKind::Network);
                self.network_filter = filter;
                self.restore_selection(&selected, ResourceKind::Network);
            }
            Tab::Volume => {
                let selected = self.selected_resource_id(ResourceKind::Volume);
                self.volume_filter = filter;
                self.restore_selection(&selected, ResourceKind::Volume);
            }
            _ => {}
        }
    }

    pub fn start_palette(&mut self) {
        commands::open_palette(self);
    }

    pub fn start_context_menu(&mut self) {
        if self.focus_target == FocusTarget::Table && !self.context_actions().is_empty() {
            self.overlay = Overlay::Context { selected: 0 };
        }
    }

    pub fn context_actions(&self) -> Vec<ContextAction> {
        match self.current_tab {
            Tab::Container => {
                let Some(index) = self.container_table_state.selected() else {
                    return Vec::new();
                };
                let containers = self.filtered_containers();
                let Some(container) = containers.get(index) else {
                    return Vec::new();
                };
                let mut actions = vec![ContextAction::Inspect, ContextAction::Logs];
                if container.state.eq_ignore_ascii_case("running") {
                    actions.extend([ContextAction::Stop, ContextAction::Restart]);
                } else {
                    actions.push(ContextAction::Start);
                }
                actions.push(ContextAction::Remove);
                actions
            }
            Tab::Image if self.selected_resource_id(ResourceKind::Image).is_some() => {
                vec![ContextAction::History, ContextAction::Remove]
            }
            Tab::Network if self.selected_resource_id(ResourceKind::Network).is_some() => {
                vec![ContextAction::Remove]
            }
            Tab::Volume if self.selected_resource_id(ResourceKind::Volume).is_some() => {
                vec![ContextAction::Remove]
            }
            _ => Vec::new(),
        }
    }

    pub fn context_request(&mut self, action: ContextAction) -> Option<TaskRequest> {
        let request = match (self.current_tab.clone(), action) {
            (Tab::Container, ContextAction::Inspect) => self
                .selected_container_id()
                .map(|id| TaskRequest::InspectContainer { id }),
            (Tab::Container, ContextAction::Logs) => {
                self.selected_container_id()
                    .map(|id| TaskRequest::ContainerLogs {
                        id,
                        follow: true,
                        timestamps: self.log_timestamps,
                    })
            }
            (Tab::Container, ContextAction::Start) => {
                self.selected_container_id()
                    .map(|id| TaskRequest::ContainerLifecycle {
                        id,
                        operation: ContainerLifecycle::Start,
                    })
            }
            (Tab::Container, ContextAction::Stop) => {
                self.selected_container_id()
                    .map(|id| TaskRequest::ContainerLifecycle {
                        id,
                        operation: ContainerLifecycle::Stop,
                    })
            }
            (Tab::Container, ContextAction::Restart) => {
                self.selected_container_id()
                    .map(|id| TaskRequest::ContainerLifecycle {
                        id,
                        operation: ContainerLifecycle::Restart,
                    })
            }
            (Tab::Container, ContextAction::Remove) => {
                self.selected_container_id()
                    .map(|id| TaskRequest::ContainerLifecycle {
                        id,
                        operation: ContainerLifecycle::Remove,
                    })
            }
            (Tab::Image, ContextAction::History) => {
                self.selected_resource_id(ResourceKind::Image).map(|id| {
                    self.execute_command(CommandSpec::new("docker").args(["history"]).arg(id))
                        .unwrap()
                })
            }
            (Tab::Image, ContextAction::Remove) => {
                self.selected_resource_id(ResourceKind::Image).map(|id| {
                    self.execute_destructive_command(
                        CommandSpec::new("docker").args(["rmi"]).arg(id),
                    )
                    .unwrap()
                })
            }
            (Tab::Network, ContextAction::Remove) => {
                self.selected_resource_id(ResourceKind::Network).map(|id| {
                    self.execute_destructive_command(
                        CommandSpec::new("docker").args(["network", "rm"]).arg(id),
                    )
                    .unwrap()
                })
            }
            (Tab::Volume, ContextAction::Remove) => {
                self.selected_resource_id(ResourceKind::Volume).map(|id| {
                    self.execute_destructive_command(
                        CommandSpec::new("docker").args(["volume", "rm"]).arg(id),
                    )
                    .unwrap()
                })
            }
            _ => None,
        };
        self.overlay = Overlay::None;
        request
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

    pub fn filtered_images(&self) -> Vec<ImageRow> {
        let filter = self.image_filter.to_lowercase();
        self.images
            .iter()
            .filter(|image| {
                filter.is_empty()
                    || image.id.to_lowercase().contains(&filter)
                    || image.tag.to_lowercase().contains(&filter)
            })
            .cloned()
            .collect()
    }

    pub fn filtered_networks(&self) -> Vec<NetworkRow> {
        let filter = self.network_filter.to_lowercase();
        self.networks
            .iter()
            .filter(|network| {
                filter.is_empty()
                    || network.id.to_lowercase().contains(&filter)
                    || network.name.to_lowercase().contains(&filter)
                    || network.driver.to_lowercase().contains(&filter)
                    || network.scope.to_lowercase().contains(&filter)
            })
            .cloned()
            .collect()
    }

    pub fn filtered_volumes(&self) -> Vec<VolumeRow> {
        let filter = self.volume_filter.to_lowercase();
        self.volumes
            .iter()
            .filter(|volume| {
                filter.is_empty()
                    || volume.name.to_lowercase().contains(&filter)
                    || volume.driver.to_lowercase().contains(&filter)
                    || volume.mountpoint.to_lowercase().contains(&filter)
            })
            .cloned()
            .collect()
    }

    pub fn set_engine_status(&mut self, status: impl Into<String>) {
        self.engine_status = status.into();
    }

    pub fn set_task_status(&mut self, status: impl Into<String>) {
        self.task_status = status.into();
    }

    pub fn set_policy(&mut self, policy: SafetyPolicy) {
        self.policy = policy;
    }

    pub fn tick(&mut self) {
        self.notifications.tick();
    }

    pub fn notify(
        &mut self,
        kind: notifications::NotificationKind,
        message: impl Into<String>,
        persistent: bool,
    ) {
        self.notifications.push(kind, message, persistent);
    }

    pub fn dismiss_notification(&mut self) {
        self.notifications.dismiss();
    }

    pub fn palette_query_push(&mut self, character: char) {
        if let Overlay::Palette { query, selected } = &mut self.overlay {
            query.push(character);
            *selected = 0;
        }
    }

    pub fn palette_query_backspace(&mut self) {
        if let Overlay::Palette { query, selected } = &mut self.overlay {
            query.pop();
            *selected = 0;
        }
    }

    pub fn palette_move(&mut self, delta: i32) {
        let Overlay::Palette { query, .. } = &self.overlay else {
            return;
        };
        let query = query.clone();
        let count =
            commands::filtered(&*self, &query, self.policy, self.task_status != "idle").len();
        if count == 0 {
            return;
        }
        let Overlay::Palette { selected, .. } = &mut self.overlay else {
            return;
        };
        *selected = ((*selected as i32 + delta).rem_euclid(count as i32)) as usize;
    }

    pub fn palette_request(&mut self) -> Option<TaskRequest> {
        let Overlay::Palette { query, selected } = &self.overlay else {
            return None;
        };
        let entries = commands::filtered(self, query, self.policy, self.task_status != "idle");
        let entry = entries.get(*selected)?;
        if !entry.enabled {
            self.notify(
                notifications::NotificationKind::Warning,
                entry
                    .reason
                    .clone()
                    .unwrap_or_else(|| "acción no disponible".to_string()),
                false,
            );
            return None;
        }
        let request = commands::request(self, entry.id);
        if !matches!(self.overlay, Overlay::Search(_))
            && !matches!(
                entry.id,
                commands::CommandId::Filter | commands::CommandId::ContextActions
            )
        {
            self.overlay = Overlay::None;
        }
        request
    }

    pub fn context_move(&mut self, delta: i32) {
        if !matches!(self.overlay, Overlay::Context { .. }) {
            return;
        }
        let count = self.context_actions().len();
        if count > 0 {
            let Overlay::Context { selected } = &mut self.overlay else {
                return;
            };
            *selected = ((*selected as i32 + delta).rem_euclid(count as i32)) as usize;
        }
    }

    pub fn context_request_selected(&mut self) -> Option<TaskRequest> {
        let Overlay::Context { selected } = self.overlay else {
            return None;
        };
        let action = self.context_actions().get(selected).copied()?;
        self.context_request(action)
    }

    pub fn focus_label(&self) -> &'static str {
        match self.focus_target {
            FocusTarget::Table => "table",
            FocusTarget::Actions => "actions",
        }
    }

    pub fn set_theme(&mut self, theme: ThemeName) {
        self.theme = theme;
    }

    pub fn cycle_theme(&mut self) -> ThemeName {
        self.theme = match self.theme {
            ThemeName::Dark => ThemeName::Light,
            ThemeName::Light => ThemeName::Mono,
            ThemeName::Mono => ThemeName::Dark,
        };
        self.theme
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

    pub fn current_tab_refresh_request(&self) -> Option<TaskRequest> {
        match self.current_tab {
            Tab::Dashboard => Some(self.dashboard_request()),
            Tab::Container => Some(TaskRequest::ListContainers),
            Tab::Image => Some(TaskRequest::ListImages),
            Tab::Network => Some(TaskRequest::ListNetworks),
            Tab::Volume => Some(TaskRequest::ListVolumes),
            Tab::Project | Tab::Help => None,
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

    pub fn push_event(&mut self, line: String, alert: Option<String>) {
        self.event_lines.push_back(line);
        while self.event_lines.len() > MAX_EVENT_LINES {
            self.event_lines.pop_front();
        }
        if let Some(alert) = alert {
            self.alerts.push_back(alert);
            while self.alerts.len() > MAX_ALERTS {
                self.alerts.pop_front();
            }
        }
    }

    pub fn start_log_filter(&mut self) {
        self.start_input("Filtro de logs:", PendingAction::LogFilter);
    }

    pub fn cycle_log_level(&mut self) {
        self.log_level_filter = match self.log_level_filter {
            None => Some(logs::LogLevel::Debug),
            Some(logs::LogLevel::Debug) => Some(logs::LogLevel::Info),
            Some(logs::LogLevel::Info) => Some(logs::LogLevel::Warn),
            Some(logs::LogLevel::Warn) => Some(logs::LogLevel::Error),
            Some(logs::LogLevel::Error) => None,
        };
    }

    pub fn toggle_log_timestamps(&mut self) {
        self.log_timestamps = !self.log_timestamps;
    }

    pub fn filtered_log_lines(&self) -> Vec<String> {
        let filter = self.log_filter.to_lowercase();
        self.log_lines
            .iter()
            .filter(|line| {
                (filter.is_empty() || line.to_lowercase().contains(&filter))
                    && logs::matches_level(line, self.log_level_filter)
            })
            .cloned()
            .collect()
    }

    pub fn queue_task(&mut self, id: u64, description: String) {
        self.task_history.queue(id, description);
    }

    pub fn start_task(&mut self, id: u64) {
        self.task_history.start(id, std::time::Instant::now());
    }

    pub fn finish_task(&mut self, id: u64, state: task_history::TaskState) {
        self.task_history
            .finish(id, state, std::time::Instant::now());
    }

    pub fn record_metrics(&mut self, at: std::time::Instant, sample: monitoring::RawSample) {
        self.latest_metrics = Some(self.metrics_history.push(at, sample));
    }

    pub fn reset_metrics(&mut self) {
        self.metrics_history = monitoring::MetricsHistory::new(120);
        self.latest_metrics = None;
    }

    pub fn set_health(&mut self, health: HealthSnapshot) {
        self.health = Some(health);
    }

    // --- Generic navigation ---
    fn next_in_list(state: &mut ListState, len: usize) {
        navigation::next_list(state, len);
    }

    fn previous_in_list(state: &mut ListState, len: usize) {
        navigation::previous_list(state, len);
    }

    fn next_in_table(state: &mut ratatui::widgets::TableState, len: usize) {
        navigation::next_table(state, len);
    }

    fn previous_in_table(state: &mut ratatui::widgets::TableState, len: usize) {
        navigation::previous_table(state, len);
    }

    pub fn next(&mut self) {
        match self.current_tab {
            Tab::Dashboard => {}
            Tab::Container => {
                if self.focus_target == FocusTarget::Table {
                    let len = self.filtered_containers().len();
                    Self::next_in_table(&mut self.container_table_state, len);
                } else {
                    Self::next_in_list(
                        &mut self.container_list_state,
                        self.container_actions.len(),
                    );
                }
            }
            Tab::Image => {
                if self.focus_target == FocusTarget::Table {
                    let len = self.filtered_images().len();
                    Self::next_in_table(&mut self.image_table_state, len);
                } else {
                    Self::next_in_list(&mut self.image_list_state, self.image_actions.len());
                }
            }
            Tab::Network => {
                if self.focus_target == FocusTarget::Table {
                    let len = self.filtered_networks().len();
                    Self::next_in_table(&mut self.network_table_state, len);
                } else {
                    Self::next_in_list(&mut self.network_list_state, self.network_actions.len());
                }
            }
            Tab::Volume => {
                if self.focus_target == FocusTarget::Table {
                    let len = self.filtered_volumes().len();
                    Self::next_in_table(&mut self.volume_table_state, len);
                } else {
                    Self::next_in_list(&mut self.volume_list_state, self.volume_actions.len());
                }
            }
            Tab::Project => {
                Self::next_in_list(&mut self.project_list_state, self.project_actions.len())
            }
            Tab::Help => {}
        }
    }

    pub fn previous(&mut self) {
        match self.current_tab {
            Tab::Dashboard => {}
            Tab::Container => {
                if self.focus_target == FocusTarget::Table {
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
                if self.focus_target == FocusTarget::Table {
                    let len = self.filtered_images().len();
                    Self::previous_in_table(&mut self.image_table_state, len);
                } else {
                    Self::previous_in_list(&mut self.image_list_state, self.image_actions.len());
                }
            }
            Tab::Network => {
                if self.focus_target == FocusTarget::Table {
                    let len = self.filtered_networks().len();
                    Self::previous_in_table(&mut self.network_table_state, len);
                } else {
                    Self::previous_in_list(
                        &mut self.network_list_state,
                        self.network_actions.len(),
                    );
                }
            }
            Tab::Volume => {
                if self.focus_target == FocusTarget::Table {
                    let len = self.filtered_volumes().len();
                    Self::previous_in_table(&mut self.volume_table_state, len);
                } else {
                    Self::previous_in_list(&mut self.volume_list_state, self.volume_actions.len());
                }
            }
            Tab::Project => {
                Self::previous_in_list(&mut self.project_list_state, self.project_actions.len())
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
            Tab::Project => Tab::Help,
            Tab::Help => Tab::Dashboard,
        };
        self.sync_focus_target();
    }

    pub fn previous_tab(&mut self) {
        self.current_tab = match self.current_tab {
            Tab::Dashboard => Tab::Help,
            Tab::Container => Tab::Dashboard,
            Tab::Image => Tab::Container,
            Tab::Network => Tab::Image,
            Tab::Volume => Tab::Network,
            Tab::Project => Tab::Volume,
            Tab::Help => Tab::Project,
        };
        self.sync_focus_target();
    }

    pub fn show_help(&mut self) {
        self.current_tab = Tab::Help;
        self.sync_focus_target();
    }

    pub fn is_help(&self) -> bool {
        matches!(self.current_tab, Tab::Help)
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

        if value.is_empty()
            && !matches!(
                action.as_ref(),
                Some(
                    PendingAction::ContainerFilter
                        | PendingAction::ImageFilter
                        | PendingAction::NetworkFilter
                        | PendingAction::VolumeFilter
                        | PendingAction::LogFilter,
                )
            )
        {
            self.push_output("[entrada cancelada: valor vacío]");
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
                timestamps: self.log_timestamps,
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
            PendingAction::ImageFilter => {
                self.image_filter = value.to_string();
                self.image_table_state.select(Some(0));
                None
            }
            PendingAction::NetworkFilter => {
                self.network_filter = value.to_string();
                self.network_table_state.select(Some(0));
                None
            }
            PendingAction::VolumeFilter => {
                self.volume_filter = value.to_string();
                self.volume_table_state.select(Some(0));
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
            PendingAction::VolumeBackup => self.volume_archive_command(value, false),
            PendingAction::VolumeRestore => self.volume_archive_command(value, true),
            // Project
            PendingAction::ProjectSetFolder => {
                self.project_folder = value.to_string();
                let folder = self.project_folder.clone();
                self.push_output(format!("[proyecto] carpeta establecida: {}", folder));
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

    fn volume_archive_command(&mut self, value: &str, restore: bool) -> Option<TaskRequest> {
        let Some((volume, archive)) = value.split_once('|') else {
            self.push_output("[volume] use: volume|archive path");
            return None;
        };
        let volume = volume.trim();
        let archive = PathBuf::from(archive.trim());
        let Some(filename) = archive.file_name().and_then(|name| name.to_str()) else {
            self.push_output("[volume] archive path must include a filename");
            return None;
        };
        let parent = archive
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."));
        self.push_output("[warning] volume backup/restore may be inconsistent; stop writers first");
        let volume_mount = format!("type=volume,source={},target=/volume", volume);
        let backup_mount = format!(
            "type=bind,source={},target=/backup",
            parent.to_string_lossy()
        );
        let mut command = CommandSpec::new("docker")
            .args(["run", "--rm", "--mount"])
            .arg(volume_mount)
            .args(["--mount"])
            .arg(backup_mount)
            .args(["alpine", "tar"]);
        command = if restore {
            command.args(["-xzf", &format!("/backup/{}", filename), "-C", "/volume"])
        } else {
            command.args([
                "-czf",
                &format!("/backup/{}", filename),
                "-C",
                "/volume",
                ".",
            ])
        };
        self.execute_mutating_command(command)
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
            NetworkAction::List => Some(TaskRequest::ListNetworks),
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
            VolumeAction::List => Some(TaskRequest::ListVolumes),
            VolumeAction::Create => {
                self.start_input("Nombre del nuevo volumen:", PendingAction::VolumeCreate)
            }
            VolumeAction::Remove => self.start_input(
                "Nombre del volumen a eliminar:",
                PendingAction::VolumeRemove,
            ),
            VolumeAction::Backup => self.start_input(
                "Volumen y archivo (volumen|ruta.tar.gz):",
                PendingAction::VolumeBackup,
            ),
            VolumeAction::Restore => self.start_input(
                "Volumen y archivo (volumen|ruta.tar.gz):",
                PendingAction::VolumeRestore,
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

    pub fn execute_selected(&mut self) -> Option<TaskRequest> {
        match self.current_tab {
            Tab::Dashboard => None,
            Tab::Container => {
                if self.focus_target == FocusTarget::Table {
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
                if self.focus_target == FocusTarget::Table {
                    let images = self.filtered_images();
                    if let Some(index) = self.image_table_state.selected()
                        && let Some(image) = images.get(index)
                    {
                        self.push_output(format!(
                            "[image] {} | {} | {} bytes",
                            image.id, image.tag, image.size
                        ));
                    }
                    return None;
                }
                if let Some(i) = self.image_list_state.selected()
                    && let Some(action) = self.image_actions.get(i).cloned()
                {
                    return self.run_image_action(&action);
                }
                None
            }
            Tab::Network => {
                if self.focus_target == FocusTarget::Table {
                    let networks = self.filtered_networks();
                    if let Some(index) = self.network_table_state.selected()
                        && let Some(network) = networks.get(index)
                    {
                        self.push_output(format!(
                            "[network] {} | {} | {}",
                            network.name, network.driver, network.scope
                        ));
                    }
                    return None;
                }
                if let Some(i) = self.network_list_state.selected()
                    && let Some(action) = self.network_actions.get(i).cloned()
                {
                    return self.run_network_action(&action);
                }
                None
            }
            Tab::Volume => {
                if self.focus_target == FocusTarget::Table {
                    let volumes = self.filtered_volumes();
                    if let Some(index) = self.volume_table_state.selected()
                        && let Some(volume) = volumes.get(index)
                    {
                        self.push_output(format!(
                            "[volume] {} | {} | {}",
                            volume.name, volume.driver, volume.mountpoint
                        ));
                    }
                    return None;
                }
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
        app.focus_target = FocusTarget::Actions;
        app.container_list_state
            .select(Some(app.container_actions.len() - 1));

        app.next();

        assert_eq!(app.container_list_state.selected(), Some(0));
    }

    #[test]
    fn previous_wraps_to_last_container_action() {
        let mut app = App::new();
        app.focus_target = FocusTarget::Actions;
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
            app.output_lines.back().map(String::as_str),
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
    fn general_output_buffer_evicts_oldest_lines_at_capacity() {
        let mut app = App::new();
        app.set_output_capacity(2);
        app.push_output("first");
        app.push_output("second");
        app.push_output("third");

        assert_eq!(app.output_lines.len(), 2);
        assert_eq!(app.output_lines.front().map(String::as_str), Some("second"));
        assert_eq!(app.output_lines.back().map(String::as_str), Some("third"));
    }

    #[test]
    fn general_output_can_be_cleared_and_scrolled() {
        let mut app = App::new();
        app.push_output("first");
        app.push_output("second");
        app.scroll_output_home();
        assert_eq!(app.output_scroll, 0);
        app.scroll_output_down(1);
        assert_eq!(app.output_scroll, 1);
        app.clear_output();
        assert!(app.output_lines.is_empty());
        assert_eq!(app.output_scroll, 0);
    }

    #[test]
    fn event_history_and_alerts_are_bounded() {
        let mut app = App::new();
        for index in 0..600 {
            app.push_event(format!("event {}", index), Some(format!("alert {}", index)));
        }

        assert_eq!(app.event_lines.len(), 500);
        assert_eq!(app.alerts.len(), 100);
        assert_eq!(
            app.event_lines.front().map(String::as_str),
            Some("event 100")
        );
        assert_eq!(app.alerts.front().map(String::as_str), Some("alert 500"));
    }

    #[test]
    fn disk_usage_requests_are_read_only() {
        let request = TaskRequest::DiskUsage { preview: true };
        assert_eq!(request.mutation(), Mutation::ReadOnly);
    }

    #[test]
    fn volume_backup_requires_mutation_confirmation() {
        let mut app = App::new();
        app.current_tab = Tab::Volume;
        app.focus_target = FocusTarget::Actions;
        app.volume_list_state.select(Some(3));
        app.execute_selected();
        app.input_buffer = "data|/tmp/data.tar.gz".to_string();

        let request = app.confirm_input().expect("backup request");

        assert_eq!(request.mutation(), Mutation::Mutating);
        assert!(
            app.output_lines
                .back()
                .is_some_and(|line| line.contains("inconsistent"))
        );
    }

    #[test]
    fn empty_filter_input_restores_the_full_list() {
        let mut app = App::new();
        app.start_container_filter();
        app.input_buffer = "api".to_string();
        app.confirm_input();
        assert_eq!(app.container_filter, "api");

        app.start_container_filter();
        app.confirm_input();

        assert!(app.container_filter.is_empty());
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
        app.focus_target = FocusTarget::Actions;
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
        app.focus_target = FocusTarget::Actions;
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
        app.focus_target = FocusTarget::Actions;
        app.image_list_state.select(Some(2));

        let request = app.execute_selected().expect("image list request");

        assert!(matches!(request, TaskRequest::ListImages));
    }

    #[test]
    fn image_tab_toggles_between_table_and_action_focus() {
        let mut app = App::new();
        app.current_tab = Tab::Image;
        app.set_images(vec![ImageRow {
            id: "abc123".to_string(),
            tag: "demo:latest".to_string(),
            size: 1024,
        }]);

        app.focus_target = FocusTarget::Actions;
        app.focus_next();
        app.execute_selected();

        assert_eq!(app.focus_target, FocusTarget::Table);
        assert!(
            app.output_lines
                .back()
                .is_some_and(|line| line.contains("demo:latest"))
        );
    }

    #[test]
    fn network_and_volume_list_actions_use_bollard_requests() {
        let mut app = App::new();
        app.current_tab = Tab::Network;
        app.focus_target = FocusTarget::Actions;
        app.network_list_state.select(Some(0));
        assert!(matches!(
            app.execute_selected(),
            Some(TaskRequest::ListNetworks)
        ));
        app.set_networks(vec![NetworkRow {
            id: "net-id".to_string(),
            name: "bridge".to_string(),
            driver: "bridge".to_string(),
            scope: "local".to_string(),
        }]);
        app.focus_target = FocusTarget::Actions;
        app.focus_next();
        app.execute_selected();
        assert_eq!(app.focus_target, FocusTarget::Table);

        app.current_tab = Tab::Volume;
        app.focus_target = FocusTarget::Actions;
        app.volume_list_state.select(Some(0));
        assert!(matches!(
            app.execute_selected(),
            Some(TaskRequest::ListVolumes)
        ));
        app.set_volumes(vec![VolumeRow {
            name: "data".to_string(),
            driver: "local".to_string(),
            mountpoint: "/var/lib/data".to_string(),
        }]);
        app.focus_target = FocusTarget::Actions;
        app.focus_next();
        app.execute_selected();
        assert_eq!(app.focus_target, FocusTarget::Table);
    }

    #[test]
    fn all_resource_tables_support_filters() {
        let mut app = App::new();
        app.set_images(vec![ImageRow {
            id: "abc".to_string(),
            tag: "web:latest".to_string(),
            size: 1,
        }]);
        app.set_networks(vec![NetworkRow {
            id: "net".to_string(),
            name: "frontend".to_string(),
            driver: "bridge".to_string(),
            scope: "local".to_string(),
        }]);
        app.set_volumes(vec![VolumeRow {
            name: "database".to_string(),
            driver: "local".to_string(),
            mountpoint: "/data".to_string(),
        }]);
        app.image_filter = "web".to_string();
        app.network_filter = "front".to_string();
        app.volume_filter = "data".to_string();

        assert_eq!(app.filtered_images().len(), 1);
        assert_eq!(app.filtered_networks().len(), 1);
        assert_eq!(app.filtered_volumes().len(), 1);
    }

    #[test]
    fn tab_navigation_cycles_resource_tabs() {
        let mut app = App::new();

        app.next_tab();
        assert_eq!(app.current_tab, Tab::Image);
        app.previous_tab();
        assert_eq!(app.current_tab, Tab::Container);
    }

    #[test]
    fn entering_resource_tabs_requests_only_that_tab() {
        let mut app = App::new();
        app.next_tab();
        assert_eq!(
            app.current_tab_refresh_request().unwrap().description(),
            "list images"
        );
        app.next_tab();
        assert_eq!(
            app.current_tab_refresh_request().unwrap().description(),
            "list networks"
        );
        app.next_tab();
        assert_eq!(
            app.current_tab_refresh_request().unwrap().description(),
            "list volumes"
        );
    }

    #[test]
    fn incremental_search_filters_and_escape_restores_previous_filter() {
        let mut app = App::new();
        app.set_containers(vec![
            ContainerRow {
                id: "one".into(),
                name: "api".into(),
                image: "demo".into(),
                state: "running".into(),
                status: "Up".into(),
            },
            ContainerRow {
                id: "two".into(),
                name: "worker".into(),
                image: "demo".into(),
                state: "exited".into(),
                status: "Exited".into(),
            },
        ]);
        app.start_search();
        app.search_push('a');
        app.search_push('p');
        assert_eq!(app.filtered_containers().len(), 1);
        app.cancel_search();
        assert!(app.container_filter.is_empty());
        assert_eq!(app.filtered_containers().len(), 2);
    }

    #[test]
    fn selection_is_preserved_by_resource_identity() {
        let mut app = App::new();
        app.set_images(vec![
            ImageRow {
                id: "first".into(),
                tag: "one".into(),
                size: 1,
            },
            ImageRow {
                id: "second".into(),
                tag: "two".into(),
                size: 2,
            },
        ]);
        app.image_table_state.select(Some(1));
        app.set_images(vec![
            ImageRow {
                id: "second".into(),
                tag: "two".into(),
                size: 2,
            },
            ImageRow {
                id: "first".into(),
                tag: "one".into(),
                size: 1,
            },
        ]);
        assert_eq!(
            app.selected_resource_id(ResourceKind::Image).as_deref(),
            Some("second")
        );
    }

    #[test]
    fn context_actions_follow_container_state_and_keep_request_policy() {
        let mut app = App::new();
        app.set_containers(vec![ContainerRow {
            id: "container".into(),
            name: "api".into(),
            image: "demo".into(),
            state: "running".into(),
            status: "Up".into(),
        }]);
        assert!(app.context_actions().contains(&ContextAction::Stop));
        assert!(!app.context_actions().contains(&ContextAction::Start));
        let request = app.context_request(ContextAction::Remove).unwrap();
        assert_eq!(request.mutation(), Mutation::Destructive);
    }
}
