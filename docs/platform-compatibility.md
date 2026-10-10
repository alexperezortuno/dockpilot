# Platform Compatibility

| Target | Release build | Runtime validation |
| --- | --- | --- |
| Linux x86_64 (`x86_64-unknown-linux-gnu`) | CI matrix and release workflow | Must be recorded after running the binary on Linux |
| macOS Apple Silicon (`aarch64-apple-darwin`) | CI matrix and release workflow | Must be recorded after running the binary on Apple Silicon |
| macOS Intel (`x86_64-apple-darwin`) | CI matrix and release workflow | Must be recorded after running the binary on Intel macOS |
| Windows x86_64 (`x86_64-pc-windows-msvc`) | CI matrix and release workflow | Must be recorded after running the binary on Windows |

Compilation and `--version` checks prove packaging, not full platform compatibility. Before publishing the beta, run the functional smoke tests on every platform and update this table with the actual OS versions, Docker versions, and results. Do not describe a target as tested based only on a successful cross-platform build.
