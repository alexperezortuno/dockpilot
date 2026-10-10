# Troubleshooting

## Docker is disconnected

Check the daemon and context outside Dockpilot:

```bash
```

Dockpilot keeps running when Docker is unavailable and shows a disconnected state. Check socket permissions and `DOCKER_HOST`/context selection without exposing credentials.

## The TUI does not fit

Dockpilot supports compact layouts and terminals down to the sizes covered by tests. Hide Output with `o`, hide Details with `D`, and enable compact layout with `C`.

## A task is stuck

Press `x` to cancel the active task. Long-running streams such as logs and Docker events are cancellable and use bounded buffers.

## Installation fails

Confirm the archive matches your OS and architecture, download `SHA256SUMS.txt` from the same release, and verify the checksum before extracting. Do not pipe an unreviewed script directly to a shell.
