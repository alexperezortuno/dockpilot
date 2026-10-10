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
