# Dockpilot

Dockpilot is a keyboard-first Rust TUI for managing Docker Engine and Docker Compose workflows. The project is intentionally scoped to Docker only: Podman, Kubernetes, generic container-engine abstractions, and a web dashboard are out of scope.

## Current status

The project is in **Phase 1: Safe foundation**.

Completed:

- DS-001 baseline report
- DS-002 behavior-preserving modularization:
  - `src/app/` application state and actions
  - `src/docker/` Docker command execution
  - `src/tui/` terminal rendering
- DS-003 RAII terminal guard with terminal restoration tests
- DS-004 explicit argument execution with metacharacter and spaced-path tests

The current TUI preserves the original tabbed interface for containers, images, networks, volumes, projects, machines, and help.

> Important: Docker operations are still synchronous. Async task execution is scheduled for DS-005, and destructive actions still need confirmation and read-only policy enforcement.

## Requirements

- Rust toolchain with Cargo
- Docker Engine and Docker Compose CLI for Docker operations
- Linux, macOS, or Windows where Docker and the terminal backend are supported

## Build and run

```bash
cargo build
cargo run
```

## Checks

```bash
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

Current validation includes unit tests for application navigation and empty-input cancellation.

## Controls

### Normal mode

| Key | Action |
| --- | --- |
| `Tab` | Next tab |
| `Shift+Tab` | Previous tab |
| `Up` / `Down` | Navigate actions |
| `Enter` | Execute selected action or prompt for a parameter |
| `q` / `Esc` | Quit |
| `Ctrl+C` | Quit |

### Input mode

| Key | Action |
| --- | --- |
| `Enter` | Confirm input |
| `Esc` | Cancel input |
| `Backspace` | Delete character |

## Tabs

- **Container**: start/stop Compose services, list containers, logs, create/remove, inspect process state, pause/unpause, update, and wait.
- **Image**: build, rebuild, list, remove, push, pull, save/load, and history.
- **Network**: list, create, and remove Docker networks.
- **Volume**: list, create, and remove Docker volumes.
- **Project**: set the project folder.
- **Machine**: Docker Machine operations.
- **Help**: keyboard reference.

## Architecture

```text
src/
  main.rs       Terminal setup and event loop
  app/          Application state, navigation, input, and actions
  docker/       Docker command execution
  tui/          Ratatui rendering
```

The target architecture is:

```text
TUI -> application state/actions/events -> Docker services -> Bollard or docker compose CLI
```

Phase 1 will add terminal guards, safe argument execution, asynchronous tasks, configuration, and safety policies before resource-first screens are implemented.

## Safety goals

- No shell interpolation of user input.
- Blocking Docker work stays outside the TUI event loop.
- Long-running operations support cancellation.
- Logs and output buffers are bounded.
- Destructive actions require explicit confirmation.
- Read-only contexts block mutations.
- Docker unavailability is shown as an actionable disconnected state.
- Terminal state is restored after failures.
- Secrets and credentials are never logged or stored.

## Roadmap

1. **Safe foundation**: modularization, terminal guard, safe execution, async tasks, config, policies.
2. **Container-first MVP**: dashboard, container table, filters, inspect, lifecycle, logs, stats, health.
3. **Compose, images and contexts**: Compose projects/profiles, images, SSH contexts, themes, keybindings.
4. **Operations**: networks, volumes, events, monitoring, cleanup, optional backups.
5. **CLI and release**: noninteractive JSON CLI, tests, CI, documentation, binaries.

This README must be updated when each roadmap phase is completed.
