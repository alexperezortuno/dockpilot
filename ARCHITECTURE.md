# Dockpilot v2 — Architecture

## Layers
TUI (ratatui/crossterm) -> application state/actions/events -> Docker-specific services -> Bollard (Engine API) or official `docker compose` CLI.

## Proposed tree
```text
src/
  main.rs
  app/{mod.rs,state.rs,actions.rs,events.rs}
  docker/{mod.rs,client.rs,containers.rs,images.rs,volumes.rs,networks.rs,compose.rs,contexts.rs}
  tui/{mod.rs,layout.rs,widgets/,screens/}
  config/{mod.rs,validation.rs}
  monitoring/{mod.rs,stats.rs,events.rs}
  security/{mod.rs,policy.rs,confirmation.rs}
  tasks/{mod.rs,manager.rs}
  error.rs
```

## Libraries
Rust, ratatui, crossterm, tokio, bollard, serde, toml, clap, tracing, thiserror/anyhow.

## Execution model
A single UI event loop receives keyboard and typed task-result events. Docker operations run asynchronously and publish progress through bounded channels. Streaming operations support cancellation and ring buffers.

## Compose adapter
Execute `docker compose` with `tokio::process::Command`, discrete arguments, explicit `current_dir`, and captured/streamed output. Never invoke a shell for redirection or argument interpolation.

## Contexts
Honor Docker context selection and SSH settings; verify the chosen Bollard/context integration against actual Docker CLI behavior. Do not store SSH keys. A context marked read-only is blocked at the application action layer.

## Configuration
Defaults -> TOML -> environment -> CLI flags, with validation and diagnostics. Store no secrets.

## Tests
Unit tests for policy/config/state; mockable Docker-specific service operations; opt-in Docker integration tests; terminal lifecycle smoke tests.

## Migration
Baseline existing code, modularize without changing behavior, replace unsafe command execution, add async task handling, then implement resource-first screens.
