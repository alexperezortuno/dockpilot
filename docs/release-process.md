# Release Process

Dockpilot follows SemVer. Beta versions use `MAJOR.MINOR.PATCH-beta.N`; beta releases may change behavior before the stable `MAJOR.MINOR.PATCH` release. The package version in `Cargo.toml` is the single source of truth.

1. Update `Cargo.toml`, `Cargo.lock`, `CHANGELOG.md`, and release notes.
2. Run the required checks locally and review the release candidate branch.
3. Open a pull request and wait for CI and human approval.
4. After approval, create and push a matching tag, for example `v0.5.0-beta.1`.
5. GitHub Actions builds all targets, verifies artifacts and checksums, creates provenance attestations, and opens a prerelease.
6. Review the generated GitHub Release manually. Do not publish from a pull request.

This repository does not create tags or releases automatically during development.
