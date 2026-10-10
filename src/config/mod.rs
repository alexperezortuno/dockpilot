use clap::{Parser, ValueEnum};
use serde::{Deserialize, Serialize};
use std::{env, fmt, path::PathBuf, str::FromStr};

const DEFAULT_CONFIG_FILE: &str = "config.toml";
const MIN_POLL_INTERVAL_MS: u64 = 10;
const MAX_POLL_INTERVAL_MS: u64 = 5_000;
const DEFAULT_OUTPUT_CAPACITY: usize = 2_000;
const MAX_OUTPUT_CAPACITY: usize = 100_000;
const PREFERENCES_FILE: &str = "dockpilot.preferences.toml";

#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq, Eq, Serialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
#[value(rename_all = "lower")]
pub enum ThemeName {
    #[default]
    Dark,
    Light,
    Mono,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Shortcuts {
    pub cancel_task: char,
    pub toggle_focus: char,
    pub refresh: char,
    pub theme: char,
}

impl Default for Shortcuts {
    fn default() -> Self {
        Self {
            cancel_task: 'x',
            toggle_focus: 'm',
            refresh: 'r',
            theme: 't',
        }
    }
}

#[derive(Debug, Parser, Default)]
#[command(name = "dockpilot", about = "Docker management TUI")]
pub struct Cli {
    /// Optional TOML configuration file. Without this flag, config.toml is used if present.
    #[arg(long, value_name = "PATH")]
    pub config: Option<PathBuf>,

    /// Compose project folder and default Docker working directory.
    #[arg(long, value_name = "PATH")]
    pub project_folder: Option<PathBuf>,

    /// Event polling interval in milliseconds.
    #[arg(long, value_name = "MILLISECONDS")]
    pub poll_interval_ms: Option<u64>,

    /// Maximum general output lines retained by the TUI.
    #[arg(long, value_name = "LINES")]
    pub output_capacity: Option<usize>,

    /// Enable safe-mode policy defaults.
    #[arg(long, value_name = "BOOL")]
    pub safe_mode: Option<bool>,

    /// Block mutating actions.
    #[arg(long, value_name = "BOOL")]
    pub read_only: Option<bool>,

    /// Docker context name.
    #[arg(long, value_name = "NAME")]
    pub docker_context: Option<String>,
    /// Interface theme.
    #[arg(long, value_name = "NAME")]
    pub theme: Option<ThemeName>,

    /// Emit one JSON result and exit without starting the TUI.
    #[arg(long)]
    pub json: bool,

    #[arg(long)]
    pub list_containers: bool,
    #[arg(long)]
    pub list_images: bool,
    #[arg(long)]
    pub list_networks: bool,
    #[arg(long)]
    pub list_volumes: bool,
    #[arg(long)]
    pub dashboard: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    pub project_folder: PathBuf,
    pub poll_interval_ms: u64,
    pub output_capacity: usize,
    pub safe_mode: bool,
    pub read_only: bool,
    pub docker_context: Option<String>,
    pub theme: ThemeName,
    pub shortcuts: Shortcuts,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Serialize)]
struct PartialConfig {
    project_folder: Option<PathBuf>,
    poll_interval_ms: Option<u64>,
    output_capacity: Option<usize>,
    safe_mode: Option<bool>,
    read_only: Option<bool>,
    docker_context: Option<String>,
    theme: Option<ThemeName>,
    shortcuts: Option<PartialShortcuts>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Serialize)]
struct PartialShortcuts {
    cancel_task: Option<String>,
    toggle_focus: Option<String>,
    refresh: Option<String>,
    theme: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigError(String);

impl ConfigError {
    fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for ConfigError {}

impl Default for Config {
    fn default() -> Self {
        Self {
            project_folder: env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            poll_interval_ms: 50,
            output_capacity: DEFAULT_OUTPUT_CAPACITY,
            safe_mode: true,
            read_only: false,
            docker_context: None,
            theme: ThemeName::default(),
            shortcuts: Shortcuts::default(),
        }
    }
}

impl Config {
    pub fn load(cli: &Cli) -> Result<Self, ConfigError> {
        let file = match cli.config.clone().or_else(default_config_path) {
            Some(path) => read_file_config(&path)?,
            None => PartialConfig::default(),
        };
        let preferences = match PathBuf::from(PREFERENCES_FILE).is_file() {
            true => read_file_config(&PathBuf::from(PREFERENCES_FILE))?,
            false => PartialConfig::default(),
        };
        let environment = environment_config()?;
        let command_line = PartialConfig {
            project_folder: cli.project_folder.clone(),
            poll_interval_ms: cli.poll_interval_ms,
            output_capacity: cli.output_capacity,
            safe_mode: cli.safe_mode,
            read_only: cli.read_only,
            docker_context: cli.docker_context.clone(),
            theme: cli.theme,
            shortcuts: None,
        };

        resolve_layers(
            Self::default(),
            file,
            preferences,
            environment,
            command_line,
        )
    }

    pub fn save_preferences(&self) -> Result<(), ConfigError> {
        let preferences = PartialConfig {
            theme: Some(self.theme),
            ..PartialConfig::default()
        };
        let contents = toml::to_string(&preferences)
            .map_err(|error| ConfigError::new(format!("cannot encode preferences: {}", error)))?;
        std::fs::write(PREFERENCES_FILE, contents).map_err(|error| {
            ConfigError::new(format!("cannot save {}: {}", PREFERENCES_FILE, error))
        })
    }

    fn validate(&self) -> Result<(), ConfigError> {
        if !(MIN_POLL_INTERVAL_MS..=MAX_POLL_INTERVAL_MS).contains(&self.poll_interval_ms) {
            return Err(ConfigError::new(format!(
                "poll_interval_ms must be between {} and {}",
                MIN_POLL_INTERVAL_MS, MAX_POLL_INTERVAL_MS
            )));
        }
        if !(1..=MAX_OUTPUT_CAPACITY).contains(&self.output_capacity) {
            return Err(ConfigError::new(format!(
                "output_capacity must be between 1 and {}",
                MAX_OUTPUT_CAPACITY
            )));
        }
        if !self.project_folder.is_dir() {
            return Err(ConfigError::new(format!(
                "project_folder is not a directory: {}",
                self.project_folder.display()
            )));
        }
        Ok(())
    }
}

fn default_config_path() -> Option<PathBuf> {
    let path = PathBuf::from(DEFAULT_CONFIG_FILE);
    path.is_file().then_some(path)
}

fn read_file_config(path: &PathBuf) -> Result<PartialConfig, ConfigError> {
    let contents = std::fs::read_to_string(path)
        .map_err(|error| ConfigError::new(format!("cannot read {}: {}", path.display(), error)))?;
    toml::from_str(&contents)
        .map_err(|error| ConfigError::new(format!("invalid TOML in {}: {}", path.display(), error)))
}

fn environment_config() -> Result<PartialConfig, ConfigError> {
    Ok(PartialConfig {
        project_folder: env::var_os("DOCKPILOT_PROJECT_FOLDER").map(PathBuf::from),
        poll_interval_ms: parse_env("DOCKPILOT_POLL_INTERVAL_MS")?,
        output_capacity: parse_env("DOCKPILOT_OUTPUT_CAPACITY")?,
        safe_mode: parse_env("DOCKPILOT_SAFE_MODE")?,
        read_only: parse_env("DOCKPILOT_READ_ONLY")?,
        docker_context: env::var("DOCKPILOT_DOCKER_CONTEXT").ok(),
        theme: None,
        shortcuts: None,
    })
}

fn parse_env<T>(name: &str) -> Result<Option<T>, ConfigError>
where
    T: FromStr,
    T::Err: fmt::Display,
{
    env::var(name)
        .ok()
        .map(|value| {
            value.parse().map_err(|error: T::Err| {
                ConfigError::new(format!("invalid {} value {:?}: {}", name, value, error))
            })
        })
        .transpose()
}

fn resolve_layers(
    mut config: Config,
    file: PartialConfig,
    preferences: PartialConfig,
    environment: PartialConfig,
    command_line: PartialConfig,
) -> Result<Config, ConfigError> {
    for layer in [file, preferences, environment, command_line] {
        if let Some(value) = layer.project_folder {
            config.project_folder = value;
        }
        if let Some(value) = layer.poll_interval_ms {
            config.poll_interval_ms = value;
        }
        if let Some(value) = layer.output_capacity {
            config.output_capacity = value;
        }
        if let Some(value) = layer.safe_mode {
            config.safe_mode = value;
        }
        if let Some(value) = layer.read_only {
            config.read_only = value;
        }
        if let Some(value) = layer.docker_context {
            config.docker_context = Some(value);
        }
        if let Some(value) = layer.theme {
            config.theme = value;
        }
        if let Some(shortcuts) = layer.shortcuts {
            apply_shortcuts(&mut config.shortcuts, shortcuts)?;
        }
    }
    config.validate()?;
    Ok(config)
}

fn apply_shortcuts(
    shortcuts: &mut Shortcuts,
    partial: PartialShortcuts,
) -> Result<(), ConfigError> {
    if let Some(value) = partial.cancel_task {
        shortcuts.cancel_task = parse_shortcut("cancel_task", &value)?;
    }
    if let Some(value) = partial.toggle_focus {
        shortcuts.toggle_focus = parse_shortcut("toggle_focus", &value)?;
    }
    if let Some(value) = partial.refresh {
        shortcuts.refresh = parse_shortcut("refresh", &value)?;
    }
    if let Some(value) = partial.theme {
        shortcuts.theme = parse_shortcut("theme", &value)?;
    }
    Ok(())
}

fn parse_shortcut(name: &str, value: &str) -> Result<char, ConfigError> {
    let mut chars = value.chars();
    match (chars.next(), chars.next()) {
        (Some(value), None) => Ok(value),
        _ => Err(ConfigError::new(format!(
            "shortcut {} must contain exactly one character",
            name
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Config, ConfigError, PartialConfig, PartialShortcuts, Shortcuts, ThemeName, resolve_layers,
    };
    use std::path::PathBuf;

    #[test]
    fn precedence_is_defaults_then_file_then_environment_then_cli() {
        let defaults = Config {
            project_folder: PathBuf::from("."),
            poll_interval_ms: 50,
            output_capacity: super::DEFAULT_OUTPUT_CAPACITY,
            safe_mode: true,
            read_only: false,
            docker_context: None,
            theme: ThemeName::Dark,
            shortcuts: Shortcuts::default(),
        };
        let file = PartialConfig {
            project_folder: Some(PathBuf::from(".")),
            poll_interval_ms: Some(100),
            output_capacity: Some(1000),
            safe_mode: Some(false),
            read_only: Some(false),
            docker_context: None,
            theme: None,
            shortcuts: None,
        };
        let environment = PartialConfig {
            project_folder: None,
            poll_interval_ms: Some(200),
            output_capacity: None,
            safe_mode: Some(true),
            read_only: Some(true),
            docker_context: None,
            theme: None,
            shortcuts: None,
        };
        let command_line = PartialConfig {
            project_folder: Some(PathBuf::from(".")),
            poll_interval_ms: Some(500),
            output_capacity: None,
            safe_mode: None,
            read_only: Some(false),
            docker_context: None,
            theme: None,
            shortcuts: None,
        };

        let config = resolve_layers(
            defaults,
            file,
            PartialConfig::default(),
            environment,
            command_line,
        )
        .unwrap();

        assert_eq!(config.poll_interval_ms, 500);
        assert!(config.safe_mode);
        assert!(!config.read_only);
    }

    #[test]
    fn invalid_poll_interval_is_rejected() {
        let result = resolve_layers(
            Config::default(),
            PartialConfig {
                poll_interval_ms: Some(1),
                ..PartialConfig::default()
            },
            PartialConfig::default(),
            PartialConfig::default(),
            PartialConfig::default(),
        );

        assert_eq!(
            result,
            Err(ConfigError::new(
                "poll_interval_ms must be between 10 and 5000"
            ))
        );
    }

    #[test]
    fn theme_and_shortcuts_are_loaded_from_layers() {
        let config = resolve_layers(
            Config::default(),
            PartialConfig {
                theme: Some(ThemeName::Light),
                shortcuts: Some(PartialShortcuts {
                    theme: Some("z".to_string()),
                    ..PartialShortcuts::default()
                }),
                ..PartialConfig::default()
            },
            PartialConfig::default(),
            PartialConfig::default(),
            PartialConfig::default(),
        )
        .unwrap();

        assert_eq!(config.theme, ThemeName::Light);
        assert_eq!(config.shortcuts.theme, 'z');
    }
}
