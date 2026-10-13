# Security

Dockpilot communicates with Docker Engine and Docker Compose only. It does not provide telemetry or store credentials.

- User input is passed as explicit process arguments; shell interpolation is not used.
- Destructive actions require confirmation.
- Read-only mode blocks mutations.
- Logs and task output are bounded.
- Long-running tasks can be cancelled.
- Terminal state is restored on exit and failure.
- Exec is blocked in read-only contexts and follows safe-mode confirmation for mutations.
- Exec commands run with the privileges configured inside the selected container; they do not run as root on the host.
- Exec uses the selected container ID and the Docker Engine connection captured when the session starts. A context change cannot silently redirect an active session.
- Interactive input/output is kept in memory for the session only. Dockpilot does not persist command history, credentials, or terminal traffic.
- `Ctrl+]` detaches the local PTY connection without intentionally terminating the remote process.
- Interactive input uses a bounded FIFO (256 KiB); saturated input is reported instead of being silently discarded.

Access to the Docker socket is highly privileged. On many systems, access to `/var/run/docker.sock` is effectively equivalent to root access. Grant Docker access only to trusted users and contexts; do not run Dockpilot with `sudo` by default.

Report security issues through GitHub's private vulnerability reporting when enabled for the repository. Do not include credentials, private keys, or Docker socket data in public reports.
