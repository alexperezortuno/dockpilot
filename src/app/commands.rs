use super::{App, FocusTarget, Overlay, Tab};
use crate::{security::SafetyPolicy, tasks::TaskRequest};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandId {
    Refresh,
    Dashboard,
    Images,
    Networks,
    Volumes,
    DiskUsage,
    CleanupPreview,
    Filter,
    InspectSelected,
    ContextActions,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandEntry {
    pub id: CommandId,
    pub label: &'static str,
    pub description: &'static str,
    pub shortcut: &'static str,
    pub enabled: bool,
    pub reason: Option<String>,
}

pub fn registry(app: &App, policy: SafetyPolicy, task_active: bool) -> Vec<CommandEntry> {
    let busy = task_active.then_some("hay una tarea en progreso".to_string());
    let resource_tab = matches!(
        app.current_tab,
        Tab::Container | Tab::Image | Tab::Network | Tab::Volume
    );
    let mut entries = vec![
        entry(
            CommandId::Refresh,
            "Refresh current resource",
            "Reload the active resource list",
            "r",
        ),
        entry(
            CommandId::Dashboard,
            "Refresh dashboard",
            "Reload engine and selected-container statistics",
            "d",
        ),
        entry(
            CommandId::Images,
            "Refresh images",
            "Reload images from Docker Engine",
            "i",
        ),
        entry(
            CommandId::Networks,
            "Refresh networks",
            "Reload networks from Docker Engine",
            "n",
        ),
        entry(
            CommandId::Volumes,
            "Refresh volumes",
            "Reload volumes from Docker Engine",
            "v",
        ),
        entry(
            CommandId::DiskUsage,
            "Inspect disk usage",
            "Read Docker disk usage",
            "u",
        ),
        entry(
            CommandId::CleanupPreview,
            "Preview cleanup",
            "Show reclaimable resources without mutating",
            "K",
        ),
        entry(
            CommandId::Filter,
            "Search current resource",
            "Filter the active resource table incrementally",
            "/",
        ),
        entry(
            CommandId::InspectSelected,
            "Inspect selected container",
            "Inspect the selected container",
            "Enter",
        ),
        entry(
            CommandId::ContextActions,
            "Context actions",
            "Open actions for the selected resource",
            "a",
        ),
    ];
    for item in &mut entries {
        if task_active && !matches!(item.id, CommandId::Filter | CommandId::ContextActions) {
            item.enabled = false;
            item.reason = busy.clone();
        }
        if matches!(item.id, CommandId::Filter) && !resource_tab {
            item.enabled = false;
            item.reason = Some("la pestaña actual no tiene recursos filtrables".to_string());
        }
        if matches!(item.id, CommandId::InspectSelected) {
            if !matches!(app.current_tab, Tab::Container) || app.focus_target != FocusTarget::Table
            {
                item.enabled = false;
                item.reason = Some("selecciona un contenedor en la tabla".to_string());
            } else if app.selected_container_id().is_none() {
                item.enabled = false;
                item.reason = Some("no hay un contenedor seleccionado".to_string());
            }
        }
        if matches!(item.id, CommandId::ContextActions) && !resource_tab {
            item.enabled = false;
            item.reason = Some("selecciona un recurso Docker".to_string());
        }
        if !policy.allows(crate::security::Mutation::ReadOnly) {
            item.enabled = false;
            item.reason = Some("el contexto no permite operaciones".to_string());
        }
    }
    entries
}

fn entry(
    id: CommandId,
    label: &'static str,
    description: &'static str,
    shortcut: &'static str,
) -> CommandEntry {
    CommandEntry {
        id,
        label,
        description,
        shortcut,
        enabled: true,
        reason: None,
    }
}

pub fn matches(entry: &CommandEntry, query: &str) -> bool {
    let query = query.to_lowercase();
    query.is_empty()
        || entry.label.to_lowercase().contains(&query)
        || entry.description.to_lowercase().contains(&query)
        || entry.shortcut.to_lowercase().contains(&query)
}

pub fn request(app: &mut App, id: CommandId) -> Option<TaskRequest> {
    match id {
        CommandId::Refresh => match app.current_tab {
            Tab::Container => Some(TaskRequest::ListContainers),
            Tab::Image => Some(TaskRequest::ListImages),
            Tab::Network => Some(TaskRequest::ListNetworks),
            Tab::Volume => Some(TaskRequest::ListVolumes),
            _ => Some(app.dashboard_request()),
        },
        CommandId::Dashboard => Some(app.dashboard_request()),
        CommandId::Images => Some(TaskRequest::ListImages),
        CommandId::Networks => Some(TaskRequest::ListNetworks),
        CommandId::Volumes => Some(TaskRequest::ListVolumes),
        CommandId::DiskUsage => Some(TaskRequest::DiskUsage { preview: false }),
        CommandId::CleanupPreview => Some(TaskRequest::DiskUsage { preview: true }),
        CommandId::Filter => {
            app.start_search();
            None
        }
        CommandId::InspectSelected => app.execute_selected(),
        CommandId::ContextActions => {
            app.start_context_menu();
            None
        }
    }
}

pub fn filtered(
    app: &App,
    query: &str,
    policy: SafetyPolicy,
    task_active: bool,
) -> Vec<CommandEntry> {
    registry(app, policy, task_active)
        .into_iter()
        .filter(|entry| matches(entry, query))
        .collect()
}

pub fn open_palette(app: &mut App) {
    app.overlay = Overlay::Palette {
        query: String::new(),
        selected: 0,
    };
}

#[cfg(test)]
mod tests {
    use super::{CommandId, filtered, registry};
    use crate::{app::App, security::SafetyPolicy};

    #[test]
    fn registry_search_is_case_insensitive() {
        let app = App::new();
        let entries = filtered(&app, "DASHBOARD", SafetyPolicy::new(false, false), false);
        assert_eq!(entries[0].id, CommandId::Dashboard);
    }

    #[test]
    fn busy_registry_explains_disabled_commands() {
        let app = App::new();
        let entries = registry(&app, SafetyPolicy::new(false, false), true);
        assert!(
            entries
                .iter()
                .find(|e| e.id == CommandId::Refresh)
                .unwrap()
                .reason
                .is_some()
        );
    }
}
