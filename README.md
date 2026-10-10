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
- DS-015 named local/remote Docker contexts with read-only enforcement
- DS-016 configurable themes, shortcuts, and persisted theme preference
- DS-017 Bollard network and volume listings with preserved action menus
- DS-018 cancellable Docker event stream with bounded local alerts
- DS-019 read-only disk usage reporting and cleanup previews
- DS-020 guarded volume backup/restore with consistency warnings
- DS-021 noninteractive JSON query CLI
- UX-001 responsive TUI completion:
  - Screen rendering lives in `src/tui/screens/` and shared rendering patterns live in `src/tui/widgets/`.
  - Resource focus is represented by one explicit, ordered `FocusTarget` instead of per-table toggles.
  - Resource details follow the selected filtered row, including empty-resource states.
   - Rendering is covered at 120x40, 80x24, 60x20, 40x12, and 20x8 terminal sizes.
- UX-002 advanced TUI interaction:
  - `:` command palette with typed commands, availability reasons, and safe dispatch.
  - `/` incremental resource search with case-insensitive filtering and ID-preserved selection.
  - `a` context actions for selected Docker resources, using existing safety confirmation.
 - bounded, expiring success/info/warning/error notifications.
- UX-003 visual observability foundations:
  - elapsed-time CPU, memory, network and block-I/O samples in bounded circular buffers.
  - periodic Bollard sampling with cancellation through the existing task manager and a CPU sparkline.
  - configurable alert thresholds and cooldowns, colored log severity, and a bounded task summary.
- UX-004 silent refresh and TUI customization:
  - typed user/background/system task origins suppress successful periodic output and notifications.
  - configurable per-resource refresh intervals, `r` manual refresh, and `R` auto-refresh toggle.
  - Output (`o`), Details (`D`), and compact (`C`) layout toggles with persisted preferences.
- UX-005 UI polish and stability:
  - conditional rendering skips idle `terminal.draw` calls while preserving event and notification invalidation.
  - background scheduling rotates across resource lists, metrics, and healthchecks without starting duplicate tasks.
  - refresh incidents are independent per task context, command-palette `j`/`k` input is preserved, and task durations are measured from start to finish.

The current TUI provides dashboard, container, image, network, volume, project, and help tabs. Legacy Docker Machine support was removed; use Docker contexts for local, TCP, or SSH engines.

> Important: The dedicated log buffer is bounded to 2,000 lines, and general command output uses the configured bounded `VecDeque`. Safety policy is configured at startup; safe mode confirms mutations, destructive actions always require confirmation, and read-only mode blocks mutations.

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

Current validation includes 62 unit tests covering application navigation, UX layout/focus rendering, dirty rendering, JSON query dispatch, guarded volume backup dispatch, disk usage policy, bounded output/events/alerts, independent refresh incidents, network/volume listing dispatch, theme and shortcut configuration, context endpoint parsing, image listing dispatch, Compose profile dispatch, dashboard requests, container filtering and inspection, bounded logs, configuration precedence, terminal restoration, safe command arguments, task cancellation, lifecycle policy, task duration, palette text input, and engine status.

UX-001 uses `Tab`/`Shift+Tab` for ordered pane focus and `Left`/`Right` for resource tabs. The `m` shortcut remains available for focus changes. The current focus is shown in the footer and selected resource details appear beside focused tables.

## CI and Integration

GitHub CI runs formatting, unit tests, Clippy, and a release build on pushes and pull requests. Docker integration tests are intentionally opt-in because they require a reachable Docker Engine:

```bash
cargo test --test docker_integration -- --ignored
```

The manual `Docker integration` workflow runs the same ignored test in GitHub Actions.

## Releases

Pushing a tag matching `v*` runs the release workflow and publishes archives for Linux x86_64, macOS Apple Silicon, and Windows x86_64. Each archive contains the `dockpilot` binary, `README.md`, and `config.example.toml`.

## JSON CLI

Use `--json` with one read-only query. The command exits without starting the TUI:

```bash
dockpilot --json --list-containers
dockpilot --json --list-images
dockpilot --json --list-networks
dockpilot --json --list-volumes
dockpilot --json --dashboard
```

JSON mode is query-only. Mutations continue to require the interactive safety and read-only policy path.

Task concurrency design and the current serialization decision are documented in `TASK_CONCURRENCY.md`.

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
| Output capacity | `DOCKPILOT_OUTPUT_CAPACITY` | `--output-capacity LINES` |
| Safe mode | `DOCKPILOT_SAFE_MODE` | `--safe-mode BOOL` |
| Read-only mode | `DOCKPILOT_READ_ONLY` | `--read-only BOOL` |
| Docker context | `DOCKPILOT_DOCKER_CONTEXT` | `--docker-context NAME` |
| Theme | `config.toml` | `--theme dark|light|mono` |
| Metrics interval/capacity | `config.toml` | `metrics_interval_ms` / `metrics_capacity` |
| Refresh/layout preferences | `config.toml` and preferences | `auto_refresh`, `*_refresh_interval_ms`, `show_output`, `show_details`, `compact_layout`, `mouse_enabled` |

Polling intervals must be between 10 and 5000 milliseconds, and the project folder must exist.
Named Docker contexts are resolved through explicit `docker context inspect` arguments and support local, TCP, and SSH endpoints. Read-only mode applies equally to the selected local or remote context.
Metrics are retained only for the configured circular capacity. Alert thresholds and cooldown use `alert_cpu_percent`, `alert_memory_percent`, and `alert_cooldown_seconds`; containers without a healthcheck are never inferred healthy.
Theme changes are cycled with the configured theme shortcut and saved to `dockpilot.preferences.toml` on exit.

## Controls

### Normal mode

| Key | Action |
| --- | --- |
| `Tab` / `Shift+Tab` | Change focus |
| `Left` / `Right` | Change resource tab |
| `Up` / `Down` / `j` / `k` | Navigate focused component |
| `Enter` | Execute selected action or prompt for a parameter |
| `x` | Cancel the active background task |
| `c` | Clear general output |
| `PageUp` / `PageDown` | Scroll general output |
| `Home` / `End` | Jump to output start/end |
| `r` | Refresh the container table |
| `R` | Toggle automatic refresh |
| `o` | Show/hide Output |
| `D` | Show/hide Details |
| `C` | Toggle compact layout |
| `f` | Filter/search containers |
| `/` | Incrementally filter the current resource table |
| `:` | Open the command palette |
| `a` | Open actions for the selected resource |
| `m` | Toggle container table/actions focus |
| `s` | Cycle container sort field |
| `p` | Pause/resume log display |
| `L` | Cycle minimum log severity filter |
| `T` | Toggle Docker log timestamps for the next log request |
| `d` | Refresh dashboard data |
| `i` | Refresh image listing |
| `t` | Cycle theme |
| `n` | Refresh network listing |
| `v` | Refresh volume listing |
| `e` | Start or cancel Docker event streaming |
| `u` | Show Docker disk usage |
| `K` | Preview cleanup without mutating resources |
| `?` | Open contextual help |
| `Backspace` | Dismiss the oldest notification |
| `q` / `Esc` | Quit |
| `Ctrl+C` | Quit |

During a confirmation prompt, press `y` to continue or `n` / `Esc` to cancel.
The `f` filter opens the existing parameter prompt. `/` applies an incremental filter to the current resource table (containers, images, networks, or volumes); `Enter` keeps it and `Esc` restores the previous filter. When a log stream is visible, `/` retains its existing log-line filtering behavior. `:` searches commands incrementally and `a` opens state-aware actions for the selected resource. To clear a resource filter, delete its search text and press `Enter`.

Notifications are transient by default. Critical errors and pending safety confirmations remain visible until dismissed with `Backspace`; detailed task output remains in the bounded output history.

Background refreshes run independently per resource interval and never chain all Docker queries. Successful background refreshes update tables silently; failures are grouped per resource/task until recovery. Metrics and healthchecks rotate fairly with list refreshes. Disable them with `R`.

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
- **Project**: set the project folder, run Compose up/down/config, and start with a selected Compose profile.
- **Network**: list networks through Bollard, create, and remove networks.
- **Volume**: list volumes through Bollard, create/remove, and guarded backup/restore volumes using `volume|archive-path` input.
- **Help**: keyboard reference.


## Architecture

```text
src/
  main.rs       Terminal setup and event loop
  app/          Application state, navigation, output buffering, input, and actions
  docker/       Docker command execution and Bollard engine connection
  tasks/        Tokio task manager and bounded task events
  tui/          Ratatui rendering, reusable layout, widgets, and screens
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
