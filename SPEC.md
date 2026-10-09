# Dockpilot v2 — Product Specification

## Objective
A fast, safe, configurable, keyboard-first Docker management TUI for local and remote Docker contexts.

## Modules
- Dashboard: engine connectivity, container counts, health, resource summary.
- Containers: list, filter, search, inspect, start/stop/restart, pause/unpause, logs, exec, stats, health and remove.
- Images: list, inspect, build, pull, push, tag, history, save/load and remove.
- Compose: discover/register projects, profiles, multiple compose files, config validation, up/down, builds, logs, restart.
- Volumes: list, inspect, create/remove, unused-resource detection, optional backup/restore.
- Networks: list, inspect, create/remove, connect/disconnect.
- Contexts: local/SSH Docker contexts, clear active host, read-only context policy.
- Monitoring: events, stats, thresholds, reconnect feedback.
- Maintenance: disk usage, cleanup preview and guarded prune.
- Settings: TOML configuration, themes, shortcuts, polling and safe mode.
- CLI (later): scriptable JSON queries and guarded mutations.

## Quality requirements
- Async and responsive TUI; cancellable long-running tasks.
- No shell interpolation of user input.
- Bounded logs, actionable errors, terminal restoration.
- Safe defaults, no telemetry, no plaintext secrets.
- Linux/macOS/Windows where Docker capabilities permit.
- Docker unavailable must not crash the interface.

## Explicit exclusions
Podman, Kubernetes, other container engines, generic engine interfaces and a web dashboard.

## MVP acceptance
Connect to active Docker context, show containers, select and inspect, stream bounded logs, enforce confirmations/read-only policy, load optional validated TOML config.
