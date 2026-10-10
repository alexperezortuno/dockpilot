# Docker Contexts

Dockpilot uses the selected Docker context or the context supplied with `--docker-context NAME`. Supported endpoints are local sockets, TCP, and SSH contexts as supported by the Docker CLI and Bollard.

```bash
docker context ls
```

Dockpilot does not store SSH keys. A read-only configuration blocks mutations after the context is selected. Context changes should be verified with `docker context inspect NAME` before starting Dockpilot.
