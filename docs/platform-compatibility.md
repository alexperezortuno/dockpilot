# Platform Compatibility

| Target | Release build | Runtime validation |
| --- | --- | --- |
| Linux x86_64 (`x86_64-unknown-linux-gnu`) | CI matrix and release workflow | Must be recorded after running the binary on Linux |
| macOS Apple Silicon (`aarch64-apple-darwin`) | CI matrix and release workflow | Must be recorded after running the binary on Apple Silicon |
| macOS Intel (`x86_64-apple-darwin`) | CI matrix and release workflow | Must be recorded after running the binary on Intel macOS |
| Windows x86_64 (`x86_64-pc-windows-msvc`) | CI matrix and release workflow | Must be recorded after running the binary on Windows |

Compilation and `--version` checks prove packaging, not full platform compatibility. Before publishing the beta, run the functional smoke tests on every platform and update this table with the actual OS versions, Docker versions, and results. Do not describe a target as tested based only on a successful cross-platform build.

## Interactive Exec Limitations

Exec uses Bollard's HTTP attach/upgrade API and crossterm's standard xterm key encoding. Linux and macOS are the primary validation targets. Windows uses the same Docker API path, but terminal resize and modified-key behavior must be validated on a real Windows console. SSH Docker contexts are supported by the existing connection layer but should be smoke-tested separately because interactive upgrade support depends on the remote transport.

The terminal emulator supports common ANSI/VT100 and xterm sequences, UTF-8 fragments, cursor movement, colors, and bounded scrollback. Kitty graphics, sixel, alternate proprietary keyboard protocols, and applications requiring a full emulator outside this subset are not guaranteed.

Detach closes Docker's local attach stream; Docker Exec does not provide a portable signal to stop or confirm the remote shell. Dockpilot therefore confirms only that the local session task completed its detach path and intentionally leaves the remote process running. Network loss may report `Failed` instead, and a remote process can continue until its own exit or container shutdown.
