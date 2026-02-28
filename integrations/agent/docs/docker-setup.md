# Docker Setup Guide

This guide explains how to run DX in Docker mode, including bootstrap, onboarding, and daily usage.

## Prerequisites

- [Docker](https://docs.docker.com/engine/install/) or [Podman](https://podman.io/getting-started/installation)
- Git

## Quick Start

### 1. Bootstrap in Docker Mode

```bash
# Clone the repository
git clone https://github.com/zeroclaw-labs/zeroclaw.git
cd dx

# Run bootstrap with Docker mode
./bootstrap.sh --docker
```

This builds the Docker image and prepares the data directory. Onboarding is **not** run by default in Docker mode.

### 2. Run Onboarding

After bootstrap completes, run onboarding inside Docker:

```bash
# Interactive onboarding (recommended for first-time setup)
./zeroclaw_install.sh --docker --interactive-onboard

# Or non-interactive with API key
./zeroclaw_install.sh --docker --api-key "sk-..." --provider openrouter
```

### 3. Start DX

#### Daemon Mode (Background Service)

```bash
# Start as a background daemon
./zeroclaw_install.sh --docker --docker-daemon

# Check logs
docker logs -f dx-daemon

# Stop the daemon
docker rm -f dx-daemon
```

#### Interactive Mode

```bash
# Run a one-off command inside the container
docker run --rm -it \
  -v ~/.dx-docker/.dx:/home/claw/.dx \
  -v ~/.dx-docker/workspace:/workspace \
  dx-bootstrap:local \
  dx agent -m "Hello, DX!"

# Start interactive CLI mode
docker run --rm -it \
  -v ~/.dx-docker/.dx:/home/claw/.dx \
  -v ~/.dx-docker/workspace:/workspace \
  dx-bootstrap:local \
  dx agent
```

## Configuration

### Data Directory

By default, Docker mode stores data in:
- `~/.dx-docker/.dx/` - Configuration files
- `~/.dx-docker/workspace/` - Workspace files

Override with environment variable:
```bash
ZEROCLAW_DOCKER_DATA_DIR=/custom/path ./bootstrap.sh --docker
```

### Pre-seeding Configuration

If you have an existing `config.toml`, you can seed it during bootstrap:

```bash
./bootstrap.sh --docker --docker-config ./my-config.toml
```

### Using Podman

```bash
ZEROCLAW_CONTAINER_CLI=podman ./bootstrap.sh --docker
```

## Common Commands

| Task | Command |
|------|---------|
| Start daemon | `./zeroclaw_install.sh --docker --docker-daemon` |
| View daemon logs | `docker logs -f dx-daemon` |
| Stop daemon | `docker rm -f dx-daemon` |
| Run one-off agent | `docker run --rm -it ... dx agent -m "message"` |
| Interactive CLI | `docker run --rm -it ... dx agent` |
| Check status | `docker run --rm -it ... dx status` |
| Start channels | `docker run --rm -it ... dx channel start` |

Replace `...` with the volume mounts shown in [Interactive Mode](#interactive-mode).

## Reset Docker Environment

To completely reset your Docker DX environment:

```bash
./bootstrap.sh --docker --docker-reset
```

This removes:
- Docker containers
- Docker networks
- Docker volumes
- Data directory (`~/.dx-docker/`)

## Troubleshooting

### "dx: command not found"

This error occurs when trying to run `dx` directly on the host. In Docker mode, you must run commands inside the container:

```bash
# Wrong (on host)
dx agent

# Correct (inside container)
docker run --rm -it \
  -v ~/.dx-docker/.dx:/home/claw/.dx \
  -v ~/.dx-docker/workspace:/workspace \
  dx-bootstrap:local \
  dx agent
```

### No Containers Running After Bootstrap

Running `./bootstrap.sh --docker` only builds the image and prepares the data directory. It does **not** start a container. To start DX:

1. Run onboarding: `./zeroclaw_install.sh --docker --interactive-onboard`
2. Start daemon: `./zeroclaw_install.sh --docker --docker-daemon`

### Container Fails to Start

Check Docker logs for errors:
```bash
docker logs dx-daemon
```

Common issues:
- Missing API key: Run onboarding with `--api-key` or edit `config.toml`
- Permission issues: Ensure Docker has access to the data directory

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `ZEROCLAW_DOCKER_DATA_DIR` | Data directory path | `~/.dx-docker` |
| `ZEROCLAW_DOCKER_IMAGE` | Docker image name | `dx-bootstrap:local` |
| `ZEROCLAW_CONTAINER_CLI` | Container CLI (docker/podman) | `docker` |
| `ZEROCLAW_DOCKER_DAEMON_NAME` | Daemon container name | `dx-daemon` |
| `ZEROCLAW_DOCKER_CARGO_FEATURES` | Build features | (empty) |

## Related Documentation

- [Quick Start](../README.md#quick-start)
- [Configuration Reference](config-reference.md)
- [Operations Runbook](operations-runbook.md)
