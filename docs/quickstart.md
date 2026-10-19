# Quickstart

1. Install Dockpilot and confirm `dockpilot --version`.
2. Ensure the Docker Engine is running and your user can access its configured socket or context.
3. Start the TUI with `dockpilot`.
4. Select a container with the arrow keys or `j`/`k`.
5. Press `Enter` to inspect it, or `a` for context actions.
6. Use `?` for contextual help.

## Interactive Exec

1. Start a running container, for example `docker run -d --name dockpilot-exec-test alpine:latest sleep 3600`.
2. Select the container and press `a`, then choose `Exec into container (interactive)`.
3. Choose `/bin/sh`, `/bin/bash`, or `custom command`. Bash automatically falls back to `/bin/sh` when unavailable.
4. Try `pwd`, `ls`, and `echo hello`. `Ctrl+C` is sent to the process in the container; `Ctrl+]` detaches without stopping the container.
5. Remove only the temporary test container after validation: `docker rm -f dockpilot-exec-test`.

Exec requires a running container and a Docker Engine connection that supports the attach/upgrade endpoint. It emulates a bounded xterm-compatible terminal; proprietary terminal protocols and graphical terminal output are not supported.

The first beta is intentionally Docker-only. Podman, Kubernetes, web dashboards, and generic engine abstractions are not supported.
