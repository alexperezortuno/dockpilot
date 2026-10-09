# Dockpilot — Agent Instructions

## Scope
Build a Rust TUI exclusively for Docker Engine and Docker Compose. Do not implement Podman, Kubernetes or multi-engine abstractions.

## Mandatory rules
- Read SPEC.md, ARCHITECTURE.md, ROADMAP.md and TASKS.md before coding.
- Preserve working behavior incrementally; do not rewrite the entire app at once.
- Never use `sh -c` or interpolate user input into shell commands.
- Prefer Bollard for Docker Engine operations and `tokio::process::Command` with explicit arguments for Compose.
- Keep blocking work outside the TUI event loop.
- Require explicit confirmation for destructive actions; block mutations on read-only contexts.
- Do not log secrets or credentials.
- Bound log buffers, support cancellation, restore terminal state after failures.
- Add tests and update documentation with each feature.

## Agent workflow
1. Inspect the current code and establish a baseline.
2. Implement only the next task from TASKS.md.
3. Run `cargo fmt --check`, `cargo clippy` and `cargo test` when available.
4. Report files changed, test results, risks and remaining work.
5. Do not claim success if checks fail.
6. Always use the Ponytail skill for implementation.
7. Update ./README.md when completing each roadmap phase.

## graphify

This project has a knowledge graph at graphify-out/ with god nodes, community structure, and cross-file relationships.

When the user types `/graphify`, use the installed graphify skill or instructions before doing anything else.

Rules:
- For codebase questions, first run `graphify query "<question>"` when graphify-out/graph.json exists. Use `graphify path "<A>" "<B>"` for relationships and `graphify explain "<concept>"` for focused concepts. These return a scoped subgraph, usually much smaller than GRAPH_REPORT.md or raw grep output.
- Dirty graphify-out/ files are expected after hooks or incremental updates; dirty graph files are not a reason to skip graphify. Only skip graphify if the task is about stale or incorrect graph output, or the user explicitly says not to use it.
- If graphify-out/wiki/index.md exists, use it for broad navigation instead of raw source browsing.
- Read graphify-out/GRAPH_REPORT.md only for broad architecture review or when query/path/explain do not surface enough context.
- After modifying code, run `graphify update .` to keep the graph current (AST-only, no API cost).
