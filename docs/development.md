# Development

Requirements: Rust stable, Cargo, Docker Engine for integration tests, and Docker Compose for Compose workflows.

```bash
cargo fmt --check
cargo test --all-features --locked
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release --locked
```

Docker integration tests are opt-in and require a reachable Engine. Keep changes scoped to Docker Engine and Docker Compose; do not add Podman, Kubernetes, or external services.
