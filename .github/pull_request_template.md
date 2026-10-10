## Summary

Describe the change and why it is needed.

## Validation

- [ ] `cargo fmt --check`
- [ ] `cargo test --all-features --locked`
- [ ] `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] `cargo build --release --locked`
- [ ] Docker integration tested, or reason it was not run

## Safety

- [ ] No secrets or personal configuration included
- [ ] Destructive Docker operations are guarded
- [ ] Documentation updated
