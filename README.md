# Dockpilot

Dockpilot is a keyboard-first Rust TUI for managing Docker Engine and Docker Compose workflows. The project is intentionally scoped to Docker only: Podman, Kubernetes, generic container-engine abstractions, and a web dashboard are out of scope.

## Current status

The project is in **Phase 2: Container-first MVP**.

Completed:

- DS-001 baseline report
- DS-002 behavior-preserving modularization:
  - `src/app/` application state and actions
  - `src/docker/` Docker command execution
  - `src/tui/` terminal rendering
- DS-003 RAII terminal guard with terminal restoration tests
- DS-004 explicit argument execution with metacharacter and spaced-path tests
- DS-005 Tokio task manager with bounded events, progress, and cancellation
- DS-006 validated TOML configuration with environment and CLI precedence
- DS-007 safe-mode confirmations and read-only mutation blocking
- DS-008 Bollard Docker Engine connection with disconnected-state handling
- DS-009 Bollard container table with selection, sorting, filtering, and refresh
- DS-010 Bollard container inspection and guarded lifecycle operations
- DS-011 bounded Bollard log streaming with follow, pause, and search
- DS-012 dashboard health summary and selected-container CPU/memory statistics
- DS-013 Compose project actions with explicit working directories and profiles
- DS-014 Bollard image listing with task progress for image operations

The current TUI preserves the original tabbed interface for containers, images, networks, volumes, projects, machines, and help.

> Important: The dedicated log buffer is bounded to 2,000 lines; general command output is not bounded yet. Safety policy is configured at startup; safe mode confirms mutations, destructive actions always require confirmation, and read-only mode blocks mutations.

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

Current validation includes 21 unit tests covering application navigation, image listing dispatch, Compose profile dispatch, dashboard requests, container filtering and inspection, bounded logs, configuration precedence, terminal restoration, safe command arguments, task cancellation, lifecycle policy, and engine status.

## Configuration

Copy `config.example.toml` to `config.toml` for an optional local configuration. Configuration is resolved in this order:

1. Built-in defaults
2. `config.toml` or the file passed with `--config`
3. Environment variables
4. CLI flags

Supported environment variables and CLI flags:

| Setting | Environment | CLI |
| --- | --- | --- |
| Project folder | `DOCKPILOT_PROJECT_FOLDER` | `--project-folder PATH` |
| Poll interval | `DOCKPILOT_POLL_INTERVAL_MS` | `--poll-interval-ms MILLISECONDS` |
| Safe mode | `DOCKPILOT_SAFE_MODE` | `--safe-mode BOOL` |
| Read-only mode | `DOCKPILOT_READ_ONLY` | `--read-only BOOL` |

Polling intervals must be between 10 and 5000 milliseconds, and the project folder must exist.

## Controls

### Normal mode

| Key | Action |
| --- | --- |
| `Tab` | Next tab |
| `Shift+Tab` | Previous tab |
| `Up` / `Down` | Navigate actions |
| `Enter` | Execute selected action or prompt for a parameter |
| `x` | Cancel the active background task |
| `r` | Refresh the container table |
| `f` | Filter/search containers |
| `m` | Toggle container table/actions focus |
| `s` | Cycle container sort field |
| `p` | Pause/resume log display |
| `/` | Filter log lines |
| `d` | Refresh dashboard data |
| `i` | Refresh image listing |
| `q` / `Esc` | Quit |
| `Ctrl+C` | Quit |

During a confirmation prompt, press `y` to continue or `n` / `Esc` to cancel.

When the container table has focus, `Enter` inspects the selected container. Lifecycle mutations use Bollard and still pass through safe-mode confirmation and read-only policy checks.

### Input mode

| Key | Action |
| --- | --- |
| `Enter` | Confirm input |
| `Esc` | Cancel input |
| `Backspace` | Delete character |

## Tabs

- **Dashboard**: Docker Engine version, container health counts, and selected-container CPU/memory statistics.
- **Container**: start/stop named containers, stop all containers, list containers, logs, create/remove, inspect process state, pause/unpause, update, and wait.
- **Image**: list images through Bollard, build, rebuild, remove, push, pull, save/load, and history with task progress.
- **Network**: list, create, and remove Docker networks.
- **Volume**: list, create, and remove Docker volumes.
- **Project**: set the project folder, run Compose up/down/config, and start with a selected Compose profile.
- **Machine**: Docker Machine operations.
- **Help**: keyboard reference.

## Architecture

```text
src/
  main.rs       Terminal setup and event loop
  app/          Application state, navigation, input, and actions
  docker/       Docker command execution and Bollard engine connection
  tasks/        Tokio task manager and bounded task events
  tui/          Ratatui rendering
```

The target architecture is:

```text
TUI -> application state/actions/events -> Docker services -> Bollard or docker compose CLI
```

Phase 1 is complete. Phase 2 now has a non-fatal Docker Engine connection check and can proceed with resource-first screens.

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
