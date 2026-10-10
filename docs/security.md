# Security

Dockpilot communicates with Docker Engine and Docker Compose only. It does not provide telemetry or store credentials.

- User input is passed as explicit process arguments; shell interpolation is not used.
- Destructive actions require confirmation.
- Read-only mode blocks mutations.
- Logs and task output are bounded.
- Long-running tasks can be cancelled.
- Terminal state is restored on exit and failure.

Access to the Docker socket is highly privileged. On many systems, access to `/var/run/docker.sock` is effectively equivalent to root access. Grant Docker access only to trusted users and contexts; do not run Dockpilot with `sudo` by default.

Report security issues through GitHub's private vulnerability reporting when enabled for the repository. Do not include credentials, private keys, or Docker socket data in public reports.
