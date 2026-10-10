# Installation

## Release installer

On Linux or macOS, review and run the installer from a trusted checkout:

```bash
curl --fail --location https://raw.githubusercontent.com/alexperezortuno/dockpilot/v0.5.0-beta.1/scripts/install.sh -o /tmp/dockpilot-install.sh
less /tmp/dockpilot-install.sh
bash /tmp/dockpilot-install.sh 0.5.0-beta.1
```

The installer detects Linux x86_64, macOS Apple Silicon, and macOS Intel, downloads the matching GitHub Release archive, verifies `SHA256SUMS.txt`, and installs to `~/.local/bin` without `sudo`. Use `--dir DIRECTORY` or `DOCKPILOT_INSTALL_DIR` to select another directory.

The installer does not support Windows shells. Use the manual PowerShell procedure below.

## Windows PowerShell

1. Download `dockpilot-v0.5.0-beta.1-windows-x86_64.zip` and `SHA256SUMS.txt` from the release.
2. Verify the archive:

```powershell
Get-FileHash .\dockpilot-v0.5.0-beta.1-windows-x86_64.zip -Algorithm SHA256
Select-String dockpilot-v0.5.0-beta.1-windows-x86_64.zip .\SHA256SUMS.txt
```

3. Extract `dockpilot.exe` to a directory on your user PATH, such as `%USERPROFILE%\bin`.
4. Open a new PowerShell window and run `dockpilot.exe --version`.

## From source

Requires Rust and Cargo:

```bash
cargo install --path . --locked
```

## Uninstall

Remove the installed executable manually. Preferences are stored in `dockpilot.preferences.toml` in the working directory used to run Dockpilot.
