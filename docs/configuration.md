# Configuration

Copy `config.example.toml` to `config.toml`. Configuration precedence is:

1. Built-in defaults
2. TOML configuration
3. Environment variables
4. CLI flags

The example file documents polling, output capacity, safety policy, Docker context, theme, metrics, alerts, and layout preferences. Do not put credentials or private keys in configuration files.

Preferences are written to `dockpilot.preferences.toml` on exit. This file is local state and is not part of release archives.
