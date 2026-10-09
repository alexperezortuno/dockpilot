use clap::Parser;
use serde::Deserialize;
use std::{env, fmt, path::PathBuf, str::FromStr};

const DEFAULT_CONFIG_FILE: &str = "config.toml";
const MIN_POLL_INTERVAL_MS: u64 = 10;
const MAX_POLL_INTERVAL_MS: u64 = 5_000;

#[derive(Debug, Parser, Default)]
#[command(name = "dockify", about = "Docker management TUI")]
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

    /// Enable safe-mode policy defaults.
    #[arg(long, value_name = "BOOL")]
    pub safe_mode: Option<bool>,

    /// Block mutating actions.
    #[arg(long, value_name = "BOOL")]
    pub read_only: Option<bool>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    pub project_folder: PathBuf,
    pub poll_interval_ms: u64,
    pub safe_mode: bool,
    pub read_only: bool,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
struct PartialConfig {
    project_folder: Option<PathBuf>,
    poll_interval_ms: Option<u64>,
    safe_mode: Option<bool>,
    read_only: Option<bool>,
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
            safe_mode: true,
            read_only: false,
        }
    }
}

impl Config {
    pub fn load(cli: &Cli) -> Result<Self, ConfigError> {
        let file = match cli.config.clone().or_else(default_config_path) {
            Some(path) => read_file_config(&path)?,
            None => PartialConfig::default(),
        };
        let environment = environment_config()?;
        let command_line = PartialConfig {
            project_folder: cli.project_folder.clone(),
            poll_interval_ms: cli.poll_interval_ms,
            safe_mode: cli.safe_mode,
            read_only: cli.read_only,
        };

        resolve_layers(Self::default(), file, environment, command_line)
    }

    fn validate(&self) -> Result<(), ConfigError> {
        if !(MIN_POLL_INTERVAL_MS..=MAX_POLL_INTERVAL_MS).contains(&self.poll_interval_ms) {
            return Err(ConfigError::new(format!(
                "poll_interval_ms must be between {} and {}",
                MIN_POLL_INTERVAL_MS, MAX_POLL_INTERVAL_MS
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
        safe_mode: parse_env("DOCKPILOT_SAFE_MODE")?,
        read_only: parse_env("DOCKPILOT_READ_ONLY")?,
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
    environment: PartialConfig,
    command_line: PartialConfig,
) -> Result<Config, ConfigError> {
    for layer in [file, environment, command_line] {
        if let Some(value) = layer.project_folder {
            config.project_folder = value;
        }
        if let Some(value) = layer.poll_interval_ms {
            config.poll_interval_ms = value;
        }
        if let Some(value) = layer.safe_mode {
            config.safe_mode = value;
        }
        if let Some(value) = layer.read_only {
            config.read_only = value;
        }
    }
    config.validate()?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::{Config, ConfigError, PartialConfig, resolve_layers};
    use std::path::PathBuf;

    #[test]
    fn precedence_is_defaults_then_file_then_environment_then_cli() {
        let defaults = Config {
            project_folder: PathBuf::from("."),
            poll_interval_ms: 50,
            safe_mode: true,
            read_only: false,
        };
        let file = PartialConfig {
            project_folder: Some(PathBuf::from(".")),
            poll_interval_ms: Some(100),
            safe_mode: Some(false),
            read_only: Some(false),
        };
        let environment = PartialConfig {
            project_folder: None,
            poll_interval_ms: Some(200),
            safe_mode: Some(true),
            read_only: Some(true),
        };
        let command_line = PartialConfig {
            project_folder: Some(PathBuf::from(".")),
            poll_interval_ms: Some(500),
            safe_mode: None,
            read_only: Some(false),
        };

        let config = resolve_layers(defaults, file, environment, command_line).unwrap();

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
        );

        assert_eq!(
            result,
            Err(ConfigError::new(
                "poll_interval_ms must be between 10 and 5000"
            ))
        );
    }
}
