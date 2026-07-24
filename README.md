# easy-ci

Internal deployment CLI — deploy Docker-based apps from GitHub repos with auto-deploy, rollbacks, and webhooks.

## Install

### Quick install (recommended)

```bash
curl -fsSL https://raw.githubusercontent.com/deyoyk/easy-ci/main/install.sh | bash
```

This will:
- Detect your OS and architecture automatically
- Download the correct binary
- Install it to `/usr/local/bin`
- Set up a background service (systemd on Linux, launchd on macOS)

### Install specific version

```bash
curl -fsSL https://raw.githubusercontent.com/deyoyk/easy-ci/main/install.sh | bash -s -- --version 0.1.42
```

### Install without service setup

```bash
curl -fsSL https://raw.githubusercontent.com/deyoyk/easy-ci/main/install.sh | bash -s -- --no-service
```

### Install via npm

```bash
npm install -g easy-ci
```

### Manual install

Download the latest binary for your platform from [GitHub Releases](https://github.com/deyoyk/easy-ci/releases).

| Platform | Architecture | Archive |
|----------|-------------|---------|
| Linux | x86_64 | `eci-linux-x86_64.tar.gz` |
| Linux | ARM64 | `eci-linux-aarch64.tar.gz` |
| macOS | x86_64 | `eci-darwin-x86_64.tar.gz` |
| macOS | ARM64 (Apple Silicon) | `eci-darwin-aarch64.tar.gz` |
| Windows | x86_64 | `eci-windows-x86_64.zip` |

```bash
# Example for Linux x86_64
curl -fsSL -o eci.tar.gz https://github.com/deyoyk/easy-ci/releases/latest/download/eci-linux-x86_64.tar.gz
tar xzf eci.tar.gz
sudo mv eci-linux-x86_64 /usr/local/bin/eci
chmod +x /usr/local/bin/eci
```

## Uninstall

```bash
curl -fsSL https://raw.githubusercontent.com/deyoyk/easy-ci/main/uninstall.sh | bash
```

## Getting Started

```bash
# Initialize with your GitHub token and Docker socket
eci init

# Deploy a GitHub repo
eci deploy owner/repo

# Deploy with a database
eci deploy owner/repo --db postgres

# Deploy with a specific port
eci deploy owner/repo --port 8080

# Deploy and watch for changes (auto-redeploy on new commits)
eci deploy owner/repo --watch
```

## Commands

| Command | Description |
|---------|-------------|
| `eci init` | Configure GitHub token and Docker host |
| `eci project create` | Create a new project |
| `eci project list` | List all projects |
| `eci project delete <name>` | Delete a project |
| `eci deploy <repo>` | Deploy a GitHub repo as a Docker container |
| `eci apps` | List all deployed apps |
| `eci logs <name>` | View container logs |
| `eci stop <name>` | Stop a running container |
| `eci remove <name>` | Remove an app and its container |
| `eci status <name>` | Show app status |
| `eci history <name>` | Show deployment history |
| `eci rollback <name>` | Rollback to previous version |
| `eci dashboard` | Launch the TUI dashboard |

### Deploy Options

```
eci deploy <owner/repo> [OPTIONS]

Options:
  -n, --name <NAME>       App name (default: repo name)
  --project <PROJECT>     Project to deploy to
  -d, --db <TYPE>         Provision a database (postgres, mongo, redis, mysql)
  --port <PORT>           Expose a port
  --watch                 Auto-redeploy on new commits
```

## Background Service

The installer sets up a background service that runs `eci dashboard` on startup.

### Linux (systemd)

```bash
# Start the service
systemctl --user start eci

# Stop the service
systemctl --user stop eci

# Check status
systemctl --user status eci

# View logs
journalctl --user -u eci -f

# Enable on boot (enabled by default)
systemctl --user enable eci
```

### macOS (launchd)

```bash
# Start the service
launchctl start com.deyoyk.eci

# Stop the service
launchctl stop com.deyoyk.eci

# View logs
tail -f ~/.eci/logs/service.log
```

## Configuration

Configuration is stored at `~/.eci/config.toml`:

```toml
[github]
token = "ghp_xxxxxxxxxxxx"
default_org = ""

[docker]
host = "unix:///var/run/docker.sock"

[deploy]
health_check_timeout_secs = 60
auto_rollback_on_unhealthy = true
```

### Environment Variables

- `RUST_LOG` — Set log level (e.g., `debug`, `info`, `warn`)
- `ECI_CONFIG_DIR` — Override config directory (default: `~/.eci`)

## Supported Platforms

| OS | Architecture | Status |
|----|-------------|--------|
| Linux | x86_64 | Supported |
| Linux | ARM64 | Supported |
| macOS | x86_64 | Supported |
| macOS | ARM64 (Apple Silicon) | Supported |
| Windows | x86_64 | Supported |

## Development

```bash
# Build
cargo build --release

# Run tests
cargo test --workspace

# Lint
cargo clippy --workspace -- -D warnings

# Format
cargo fmt --all
```

## License

MIT License — see [LICENSE](LICENSE) for details.
