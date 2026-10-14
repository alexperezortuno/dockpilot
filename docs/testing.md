# Testing

Unit and TUI tests run without Docker:

```bash
cargo test --all-features --locked
```

The ignored integration test requires Docker:

```bash
cargo test --test docker_integration --locked -- --ignored
```

Integration tests must use temporary, clearly named resources and clean them up. Never run destructive tests against a user's existing containers or volumes. Release CI additionally builds and verifies `dockpilot --version` on Linux x86_64, macOS Apple Silicon, macOS Intel, and Windows x86_64.

Unified Logs unit coverage includes combined filters, stable ordering, bounded viewport caching, search navigation state, follow/pause state, buffer eviction, fragmented UTF-8 lines, line-size limits, TXT/JSONL export, and invalid timestamps. The Docker integration suite should be extended with temporary containers that emit stdout/stderr fragments and a Compose project when a reachable Engine is available.

Compose workspace unit coverage includes official-label grouping, Docker-context identity, multiple replicas, stopped containers, local folders with spaces, expansion, health/no-healthcheck rendering, and selection preservation. Discovery is read-only and inspects only containers in the active Engine context.
