# Contributing

Fork or branch from `develop`, make a focused change, and open a pull request. Do not push tags or publish releases from a pull request.

Before opening a pull request:

```bash
cargo fmt --check
cargo test --all-features --locked
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release --locked
```

Keep the project scoped to Docker Engine and Docker Compose. Add tests and documentation with behavior changes. Do not include secrets, personal preferences, generated binaries, or Docker resources in commits.
