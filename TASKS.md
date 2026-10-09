# Dockpilot v2 — Implementation Tasks

## P0
- [x] DS-001: Baseline current repository, dependencies, build/test status and known defects. Deliver report; no refactor.
- [x] DS-002: Extract app state, Docker operations and TUI rendering into modules while preserving behavior.
- [x] DS-003: Introduce RAII terminal guard and failure recovery tests.
- [x] DS-004: Replace `sh -c` with typed, explicit argument execution; test metacharacters and paths with spaces.
- [ ] DS-005: Introduce Tokio tasks, bounded channels, progress and cancellation; prove navigation stays responsive.
- [ ] DS-006: Add validated TOML configuration, defaults, precedence and example file.
- [ ] DS-007: Add destructive-action confirmation and read-only context enforcement.

## P1
- [ ] DS-008: Bollard Docker Engine connection and disconnected state.
- [ ] DS-009: Container table with selection, sorting, filtering and search.
- [ ] DS-010: Inspect and guarded container lifecycle operations.
- [ ] DS-011: Follow/search/pause logs with bounded ring buffer.
- [ ] DS-012: Dashboard, health and CPU/memory statistics.

## P2
- [ ] DS-013: Compose project management with correct working directory and profiles.
- [ ] DS-014: Image management and progress.
- [ ] DS-015: Local/remote Docker contexts and read-only policy.
- [ ] DS-016: Themes, shortcuts and preference persistence.

## P3
- [ ] DS-017: Networks and volumes.
- [ ] DS-018: Docker events and local monitoring alerts.
- [ ] DS-019: Disk usage and safe cleanup previews.
- [ ] DS-020: Optional volume backup/restore with consistency warnings.

## P4
- [ ] DS-021: Noninteractive CLI and JSON output.
- [ ] DS-022: CI, integration tests, documentation and release packaging.

## First agent assignment
Implement DS-001 only. Inspect the repository, run baseline checks where available, and report shell interpolation, blocking subprocess calls, hardcoded parameters, broken output scrolling and Compose working-directory behavior. Do not refactor before reporting findings.
