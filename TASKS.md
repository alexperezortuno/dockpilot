# Dockpilot v2 — Implementation Tasks

## P0
- [x] DS-001: Baseline current repository, dependencies, build/test status and known defects. Deliver report; no refactor.
- [x] DS-002: Extract app state, Docker operations and TUI rendering into modules while preserving behavior.
- [x] DS-003: Introduce RAII terminal guard and failure recovery tests.
- [x] DS-004: Replace `sh -c` with typed, explicit argument execution; test metacharacters and paths with spaces.
- [x] DS-005: Introduce Tokio tasks, bounded channels, progress and cancellation; prove navigation stays responsive.
- [x] DS-006: Add validated TOML configuration, defaults, precedence and example file.
- [x] DS-007: Add destructive-action confirmation and read-only context enforcement.

## P1
- [x] DS-008: Bollard Docker Engine connection and disconnected state.
- [x] DS-009: Container table with selection, sorting, filtering and search.
- [x] DS-010: Inspect and guarded container lifecycle operations.
- [x] DS-011: Follow/search/pause logs with bounded ring buffer.
- [x] DS-012: Dashboard, health and CPU/memory statistics.

## P2
- [x] DS-013: Compose project management with correct working directory and profiles.
- [x] DS-014: Image management and progress.
- [x] DS-015: Local/remote Docker contexts and read-only policy.
- [x] DS-016: Themes, shortcuts and preference persistence.

## P3
- [x] DS-017: Networks and volumes.
- [x] DS-018: Docker events and local monitoring alerts.
- [x] DS-019: Disk usage and safe cleanup previews.
- [x] DS-020: Optional volume backup/restore with consistency warnings.

## P4
- [x] DS-021: Noninteractive CLI and JSON output.
- [x] DS-022: CI, integration tests, documentation and release packaging.

## UX
- [x] UX-006: Local system information tab with asynchronous host metrics, resource details, and refresh behavior.
- [x] UX-001: Responsive layout, explicit focus, contextual details, and adaptive TUI states.
  - Rendering extracted to `src/tui/screens/`.
  - Reusable widgets for tables, actions, details, navigation, and empty states.
  - Single ordered focus through `FocusTarget`, preserving `m`, `Tab`, `Shift+Tab`, and tab navigation.
    - Rendering tests for 120x40, 80x24, 60x20, 40x12, and 20x8, including selection-driven detail updates.
- [x] UX-002: Advanced TUI interaction.
  - Command palette `:` with typed registry, incremental search, keyboard navigation, availability reasons, and safe dispatch.
  - Incremental resource search `/` for containers, images, networks, and volumes with filter restoration and ID-preserved selection.
  - Context actions `a` derived from selected resource state and routed through existing `TaskRequest` and safety policy.
  - Bounded expiring notifications integrated with task results and persistent critical errors.
   - Tests for command filtering, search restoration, selection preservation, context policy, notification expiration, and 120x40/80x24/60x20 rendering.
- [x] UX-003: Visual observability foundations.
  - Bounded elapsed-time metric calculations, periodic Docker stats extraction, CPU sparkline, alert cooldowns, visual log classification, and typed bounded task history.
  - Configuration example documents metric retention and alert thresholds.
- [x] UX-004: Silent refresh, configurable polling, adaptive layout, and persisted preferences.
  - Background tasks carry a typed origin and suppress successful periodic output/notifications.
  - `r` refreshes manually, `R` toggles auto-refresh, and resource intervals are validated in TOML.
  - Output/details visibility and compact layout can be toggled without changing keyboard navigation.
- [x] UX-005: UI polish and stability.
  - Conditional rendering consumes an application dirty flag; keyboard/task/notification changes invalidate it.
  - Background refresh failures are grouped independently by resource/task description, with one recovery notification.
  - Rotating background scheduling prevents metrics and healthchecks from starving behind list refreshes.
  - Command palette navigation uses arrows; `j` and `k` remain text input, and task duration uses `finished - started`.
  - Added dirty-render, task duration, independent refresh incident, palette input, identity selection, and terminal-size coverage.

## First agent assignment
Implement DS-001 only. Inspect the repository, run baseline checks where available, and report shell interpolation, blocking subprocess calls, hardcoded parameters, broken output scrolling and Compose working-directory behavior. Do not refactor before reporting findings.
