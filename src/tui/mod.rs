pub mod layout;
pub mod screens;
pub mod terminal;
pub mod widgets;

use self::layout::areas;
use self::widgets::status::footer_line;
use crate::app::{App, Tab};
use crate::config::ThemeName;
use ratatui::{
    Frame,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph, Tabs},
};

#[derive(Clone, Copy)]
pub(crate) struct Palette {
    pub(crate) foreground: Color,
    pub(crate) accent: Color,
    pub(crate) selection: Color,
}

pub(crate) fn palette(theme: ThemeName) -> Palette {
    match theme {
        ThemeName::Dark => Palette {
            foreground: Color::White,
            accent: Color::Yellow,
            selection: Color::Blue,
        },
        ThemeName::Light => Palette {
            foreground: Color::Black,
            accent: Color::Green,
            selection: Color::LightBlue,
        },
        ThemeName::Mono => Palette {
            foreground: Color::Gray,
            accent: Color::White,
            selection: Color::DarkGray,
        },
    }
}

pub fn draw_app(f: &mut Frame, app: &mut App) {
    let layout = areas(
        f.area(),
        app.input_mode || matches!(app.overlay, crate::app::Overlay::Search(_)),
        app.show_output,
        app.compact_layout,
    );
    let colors = palette(app.theme);
    let tab_titles = [
        "Dashboard",
        "Container",
        "Image",
        "Network",
        "Volume",
        "Project",
        "Help",
    ];
    let selected_tab = match app.current_tab {
        Tab::Dashboard => 0,
        Tab::Container => 1,
        Tab::Image => 2,
        Tab::Network => 3,
        Tab::Volume => 4,
        Tab::Project => 5,
        Tab::Help => 6,
    };

    let tabs = Tabs::new(tab_titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" Dockpilot | Docker: {} ", app.engine_status)),
        )
        .select(selected_tab)
        .style(Style::default().fg(colors.foreground))
        .highlight_style(
            Style::default()
                .fg(colors.accent)
                .add_modifier(Modifier::BOLD),
        );
    f.render_widget(tabs, layout.header);
    f.render_widget(
        Paragraph::new(footer_line(
            app.focus_label(),
            &format!("{} | {}", app.task_status, app.refresh_status),
        )),
        layout.footer,
    );

    match app.current_tab {
        Tab::Dashboard => screens::dashboard::draw(f, app, layout.content),
        Tab::Container | Tab::Image | Tab::Network | Tab::Volume => {
            screens::resources::draw(f, app, layout.content, colors)
        }
        Tab::Project | Tab::Help => {
            screens::resources::draw_actions(f, app, layout.content, colors)
        }
    }

    if !matches!(app.current_tab, Tab::Dashboard)
        && let Some(area) = layout.input
    {
        widgets::input::draw(f, app, area);
    }
    if app.show_output {
        widgets::output::draw(f, app, layout.output);
    }
    if matches!(app.overlay, crate::app::Overlay::Search(_))
        && let Some(area) = layout.input
    {
        widgets::search_bar::draw(f, app, area);
    }
    widgets::command_palette::draw(f, app, f.area(), colors);
    widgets::context_menu::draw(f, app, f.area(), colors);
    widgets::toast::draw(f, app, f.area(), colors);
}

#[cfg(test)]
mod tests {
    use super::draw_app;
    use crate::app::{App, ContainerSort, FocusTarget, Tab};
    use crate::docker::client::{ContainerRow, ImageRow, NetworkRow, VolumeRow};
    use ratatui::{Terminal, backend::TestBackend};

    fn render(width: u16, height: u16, app: &mut App) -> String {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).expect("test terminal");
        terminal.draw(|frame| draw_app(frame, app)).expect("render");
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect()
    }

    #[test]
    fn renders_all_tabs_at_supported_sizes() {
        let sizes = [(120, 40), (80, 24), (60, 20), (40, 12), (20, 8)];
        for (width, height) in sizes {
            let mut app = App::new();
            for tab in [
                Tab::Dashboard,
                Tab::Container,
                Tab::Image,
                Tab::Network,
                Tab::Volume,
                Tab::Project,
                Tab::Help,
            ] {
                app.current_tab = tab;
                let _ = render(width, height, &mut app);
            }
        }
    }

    #[test]
    fn renders_populated_tables_and_input_at_small_sizes() {
        let mut app = App::new();
        app.set_containers(vec![ContainerRow {
            id: "container-id".into(),
            name: "api".into(),
            image: "demo:latest".into(),
            state: "running".into(),
            status: "Up".into(),
        }]);
        app.set_images(vec![ImageRow {
            id: "image-id".into(),
            tag: "demo:latest".into(),
            size: 1,
        }]);
        app.set_networks(vec![NetworkRow {
            id: "network-id".into(),
            name: "frontend".into(),
            driver: "bridge".into(),
            scope: "local".into(),
        }]);
        app.set_volumes(vec![VolumeRow {
            name: "data".into(),
            driver: "local".into(),
            mountpoint: "/data".into(),
        }]);
        app.current_tab = Tab::Container;
        app.focus_target = FocusTarget::Table;
        app.start_container_filter();
        let output = render(60, 20, &mut app);
        assert!(output.contains("api"));
    }

    #[test]
    fn details_follow_the_selected_resource() {
        let mut app = App::new();
        app.current_tab = Tab::Image;
        app.focus_target = FocusTarget::Table;
        app.set_images(vec![
            ImageRow {
                id: "first".into(),
                tag: "one:latest".into(),
                size: 1,
            },
            ImageRow {
                id: "second".into(),
                tag: "two:latest".into(),
                size: 2,
            },
        ]);
        let first = render(80, 24, &mut app);
        app.image_table_state.select(Some(1));
        let second = render(80, 24, &mut app);
        assert!(first.contains("one:latest"));
        assert!(second.contains("two:latest"));
        assert_ne!(first, second);
    }

    #[test]
    fn focus_navigation_is_explicit_and_ordered() {
        let mut app = App::new();
        assert_eq!(app.focus_target, FocusTarget::Table);
        app.focus_next();
        assert_eq!(app.focus_target, FocusTarget::Actions);
        app.focus_previous();
        assert_eq!(app.focus_target, FocusTarget::Table);
        app.toggle_container_sort();
        assert_eq!(app.container_sort, ContainerSort::Image);
    }

    #[test]
    fn renders_ux_overlays_at_requested_sizes() {
        for (width, height) in [(120, 40), (80, 24), (60, 20)] {
            let mut app = App::new();
            app.current_tab = Tab::Container;
            app.set_containers(vec![ContainerRow {
                id: "container".into(),
                name: "api".into(),
                image: "demo".into(),
                state: "running".into(),
                status: "Up".into(),
            }]);
            app.start_palette();
            let _ = render(width, height, &mut app);
            app.overlay = crate::app::Overlay::Search(crate::app::search::SearchState::new(""));
            let _ = render(width, height, &mut app);
            app.overlay = crate::app::Overlay::Context { selected: 0 };
            let _ = render(width, height, &mut app);
            app.notify(
                crate::app::notifications::NotificationKind::Info,
                "ready",
                false,
            );
            let output = render(width, height, &mut app);
            assert!(output.contains("ready"));
        }
    }

    #[test]
    fn renders_compact_layout_without_output_or_details() {
        for (width, height) in [(120, 40), (80, 24), (60, 20)] {
            let mut app = App::new();
            app.show_output = false;
            app.show_details = false;
            app.compact_layout = true;
            let _ = render(width, height, &mut app);
            app.focus_target = FocusTarget::Actions;
            let _ = render(width, height, &mut app);
        }
    }
}
