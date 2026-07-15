# easy-ci Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a professional internal CLI/TUI tool that pulls GitHub repos, builds/runs Docker containers, supports auto-deploy, rollbacks, log viewing, and database provisioning.

**Architecture:** Rust workspace with 7 focused crates. CLI via clap, TUI via ratatui, Docker via bollard, GitHub via octocrab, state via SQLite.

**Tech Stack:** Rust, clap, ratatui, crossterm, bollard, octocrab, rusqlite, reqwest, tokio, serde, toml

## Global Constraints

- Rust 2021 edition, MSRV 1.70
- CLI prefix: `eci`
- App names: unique, mandatory, alphanumeric + hyphens
- Config dir: `~/.eci/`
- State: SQLite at `~/.eci/state.db`
- Secrets: `~/.eci/secrets/<app-name>.env`
- Docker images tagged as `<app-name>:<short-sha>`
- Poll interval: 30s default
- Health check timeout: 60s default

---

## Phase 1: Project Scaffolding

### Task 1.1: Initialize Cargo Workspace

**Files:**
- Create: `Cargo.toml` (workspace root)
- Create: `crates/eci-core/Cargo.toml`
- Create: `crates/eci-core/src/lib.rs`
- Create: `crates/eci-cli/Cargo.toml`
- Create: `crates/eci-cli/src/main.rs`

**Interfaces:**
- Produces: workspace with two crates that compile

- [ ] **Step 1: Create workspace root Cargo.toml**

```toml
[workspace]
resolver = "2"
members = [
    "crates/eci-core",
    "crates/eci-cli",
]
```

- [ ] **Step 2: Create eci-core crate**

```toml
# crates/eci-core/Cargo.toml
[package]
name = "eci-core"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { version = "1", features = ["derive"] }
toml = "0.8"
rusqlite = { version = "0.31", features = ["bundled"] }
thiserror = "1"
```

- [ ] **Step 3: Create eci-core lib.rs with error types**

```rust
// crates/eci-core/src/lib.rs
pub mod error;
pub mod config;
pub mod state;
pub mod types;
```

- [ ] **Step 4: Create error module**

```rust
// crates/eci-core/src/error.rs
use thiserror::Error;

#[derive(Error, Debug)]
pub enum EciError {
    #[error("Config error: {0}")]
    Config(String),
    #[error("GitHub error: {0}")]
    GitHub(String),
    #[error("Docker error: {0}")]
    Docker(String),
    #[error("Database error: {0}")]
    Database(String),
    #[error("Deploy error: {0}")]
    Deploy(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
}

pub type Result<T> = std::result::Result<T, EciError>;
```

- [ ] **Step 5: Create eci-cli crate**

```toml
# crates/eci-cli/Cargo.toml
[package]
name = "eci-cli"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "eci"
path = "src/main.rs"

[dependencies]
eci-core = { path = "../eci-core" }
clap = { version = "4", features = ["derive"] }
tokio = { version = "1", features = ["full"] }
```

- [ ] **Step 6: Create minimal main.rs**

```rust
// crates/eci-cli/src/main.rs
use clap::Parser;

#[derive(Parser)]
#[command(name = "eci", about = "Internal CI/CD tool")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Init,
    Project {
        #[command(subcommand)]
        action: ProjectAction,
    },
}

#[derive(Subcommand)]
enum ProjectAction {
    Create,
    List,
    Delete { name: String },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Init => println!("TODO: init"),
        Commands::Project { action } => match action {
            ProjectAction::Create => println!("TODO: project create"),
            ProjectAction::List => println!("TODO: project list"),
            ProjectAction::Delete { name } => println!("TODO: project delete {}", name),
        },
    }
}
```

- [ ] **Step 7: Verify it compiles**

Run: `cargo build`
Expected: Compiles with no errors

- [ ] **Step 8: Commit**

```bash
git init
echo -e "/target\n*.pdb" > .gitignore
git add .
git commit -m "chore: initialize cargo workspace with core and cli crates"
```

---

### Task 1.2: Config and State Modules

**Files:**
- Create: `crates/eci-core/src/config.rs`
- Create: `crates/eci-core/src/state.rs`
- Create: `crates/eci-core/src/types.rs`
- Modify: `crates/eci-core/Cargo.toml` (add chrono, dirs)

**Interfaces:**
- Produces: `Config::load()`, `Config::save()`, `State::new()`, `State::create_project()`, `State::list_projects()`

- [ ] **Step 1: Update eci-core dependencies**

```toml
# crates/eci-core/Cargo.toml (add to [dependencies])
chrono = { version = "0.4", features = ["serde"] }
dirs = "5"
```

- [ ] **Step 2: Create types module**

```rust
// crates/eci-core/src/types.rs
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct App {
    pub name: String,
    pub project_name: String,
    pub repo: String,
    pub description: Option<String>,
    pub image_tag: String,
    pub container_id: Option<String>,
    pub port: Option<u16>,
    pub status: AppStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AppStatus {
    Running,
    Stopped,
    Unhealthy,
    Deploying,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deployment {
    pub id: i64,
    pub app_name: String,
    pub version: String,
    pub image_tag: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbInfo {
    pub app_name: String,
    pub db_type: String,
    pub connection_string: String,
    pub credentials_path: String,
}
```

- [ ] **Step 3: Create config module**

```rust
// crates/eci-core/src/config.rs
use crate::error::{EciError, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub github: GitHubConfig,
    pub docker: DockerConfig,
    pub deploy: DeployConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GitHubConfig {
    pub token: String,
    pub default_org: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DockerConfig {
    pub host: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DeployConfig {
    pub poll_interval_secs: u64,
    pub health_check_timeout_secs: u64,
    pub auto_rollback_on_unhealthy: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            github: GitHubConfig {
                token: String::new(),
                default_org: None,
            },
            docker: DockerConfig {
                host: "unix:///var/run/docker.sock".to_string(),
            },
            deploy: DeployConfig {
                poll_interval_secs: 30,
                health_check_timeout_secs: 60,
                auto_rollback_on_unhealthy: true,
            },
        }
    }
}

impl Config {
    pub fn config_dir() -> Result<PathBuf> {
        let home = dirs::home_dir().ok_or_else(|| EciError::Config("Cannot find home dir".into()))?;
        Ok(home.join(".eci"))
    }

    pub fn config_path() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join("config.toml"))
    }

    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = fs::read_to_string(&path)
            .map_err(|e| EciError::Config(format!("Failed to read config: {}", e)))?;
        toml::from_str(&content)
            .map_err(|e| EciError::Config(format!("Failed to parse config: {}", e)))
    }

    pub fn save(&self) -> Result<()> {
        let dir = Self::config_dir()?;
        fs::create_dir_all(&dir)?;
        let content = toml::to_string_pretty(self)
            .map_err(|e| EciError::Config(format!("Failed to serialize config: {}", e)))?;
        fs::write(Self::config_path()?, content)?;
        Ok(())
    }
}
```

- [ ] **Step 4: Create state module**

```rust
// crates/eci-core/src/state.rs
use crate::error::Result;
use crate::types::{App, AppStatus, Deployment, Project};
use chrono::Utc;
use rusqlite::{params, Connection};
use std::fs;
use std::path::PathBuf;

pub struct State {
    conn: Connection,
}

impl State {
    pub fn new() -> Result<Self> {
        let config_dir = crate::config::Config::config_dir()?;
        fs::create_dir_all(&config_dir)?;
        let db_path = config_dir.join("state.db");
        let conn = Connection::open(db_path)?;
        let state = Self { conn };
        state.init_tables()?;
        Ok(state)
    }

    fn init_tables(&self) -> Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS projects (
                name TEXT PRIMARY KEY,
                description TEXT,
                created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS apps (
                name TEXT PRIMARY KEY,
                project_name TEXT NOT NULL,
                repo TEXT NOT NULL,
                description TEXT,
                image_tag TEXT NOT NULL,
                container_id TEXT,
                port INTEGER,
                status TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                FOREIGN KEY (project_name) REFERENCES projects(name)
            );
            CREATE TABLE IF NOT EXISTS deployments (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                app_name TEXT NOT NULL,
                version TEXT NOT NULL,
                image_tag TEXT NOT NULL,
                status TEXT NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY (app_name) REFERENCES apps(name)
            );",
        )?;
        Ok(())
    }

    pub fn create_project(&self, name: &str, description: Option<&str>) -> Result<Project> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO projects (name, description, created_at) VALUES (?1, ?2, ?3)",
            params![name, description, now],
        )?;
        Ok(Project {
            name: name.to_string(),
            description: description.map(String::from),
            created_at: Utc::now(),
        })
    }

    pub fn list_projects(&self) -> Result<Vec<Project>> {
        let mut stmt = self
            .conn
            .prepare("SELECT name, description, created_at FROM projects ORDER BY name")?;
        let projects = stmt
            .query_map([], |row| {
                Ok(Project {
                    name: row.get(0)?,
                    description: row.get(1)?,
                    created_at: row.get(2)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(projects)
    }

    pub fn delete_project(&self, name: &str) -> Result<bool> {
        let rows = self
            .conn
            .execute("DELETE FROM projects WHERE name = ?1", params![name])?;
        Ok(rows > 0)
    }

    pub fn create_app(
        &self,
        name: &str,
        project_name: &str,
        repo: &str,
        description: Option<&str>,
        image_tag: &str,
    ) -> Result<App> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO apps (name, project_name, repo, description, image_tag, status, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![name, project_name, repo, description, image_tag, "deploying", now, now],
        )?;
        Ok(App {
            name: name.to_string(),
            project_name: project_name.to_string(),
            repo: repo.to_string(),
            description: description.map(String::from),
            image_tag: image_tag.to_string(),
            container_id: None,
            port: None,
            status: AppStatus::Deploying,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
    }

    pub fn list_apps(&self) -> Result<Vec<App>> {
        let mut stmt = self.conn.prepare(
            "SELECT name, project_name, repo, description, image_tag, container_id, port, status, created_at, updated_at
             FROM apps ORDER BY name",
        )?;
        let apps = stmt
            .query_map([], |row| {
                let status_str: String = row.get(7)?;
                let status = match status_str.as_str() {
                    "running" => AppStatus::Running,
                    "stopped" => AppStatus::Stopped,
                    "unhealthy" => AppStatus::Unhealthy,
                    _ => AppStatus::Deploying,
                };
                Ok(App {
                    name: row.get(0)?,
                    project_name: row.get(1)?,
                    repo: row.get(2)?,
                    description: row.get(3)?,
                    image_tag: row.get(4)?,
                    container_id: row.get(5)?,
                    port: row.get(6)?,
                    status,
                    created_at: row.get(8)?,
                    updated_at: row.get(9)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(apps)
    }

    pub fn update_app_status(&self, name: &str, status: &AppStatus) -> Result<()> {
        let status_str = match status {
            AppStatus::Running => "running",
            AppStatus::Stopped => "stopped",
            AppStatus::Unhealthy => "unhealthy",
            AppStatus::Deploying => "deploying",
        };
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE apps SET status = ?1, updated_at = ?2 WHERE name = ?3",
            params![status_str, now, name],
        )?;
        Ok(())
    }

    pub fn get_app(&self, name: &str) -> Result<Option<App>> {
        let mut stmt = self.conn.prepare(
            "SELECT name, project_name, repo, description, image_tag, container_id, port, status, created_at, updated_at
             FROM apps WHERE name = ?1",
        )?;
        let mut rows = stmt.query_map(params![name], |row| {
            let status_str: String = row.get(7)?;
            let status = match status_str.as_str() {
                "running" => AppStatus::Running,
                "stopped" => AppStatus::Stopped,
                "unhealthy" => AppStatus::Unhealthy,
                _ => AppStatus::Deploying,
            };
            Ok(App {
                name: row.get(0)?,
                project_name: row.get(1)?,
                repo: row.get(2)?,
                description: row.get(3)?,
                image_tag: row.get(4)?,
                container_id: row.get(5)?,
                port: row.get(6)?,
                status,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        })?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }
}
```

- [ ] **Step 5: Verify compilation**

Run: `cargo build`
Expected: Compiles successfully

- [ ] **Step 6: Commit**

```bash
git add crates/eci-core/
git commit -m "feat(core): add config, state, and types modules"
```

---

## Phase 2: GitHub Integration

### Task 2.1: GitHub API Client

**Files:**
- Create: `crates/eci-github/Cargo.toml`
- Create: `crates/eci-github/src/lib.rs`
- Modify: `Cargo.toml` (add to workspace members)

**Interfaces:**
- Consumes: `Config.github.token`
- Produces: `GitHubClient::new()`, `GitHubClient::list_repos()`, `GitHubClient::clone_repo()`

- [ ] **Step 1: Create eci-github crate**

```toml
# crates/eci-github/Cargo.toml
[package]
name = "eci-github"
version = "0.1.0"
edition = "2021"

[dependencies]
eci-core = { path = "../eci-core" }
octocrab = "0.39"
tokio = { version = "1", features = ["full"] }
git2 = "0.19"
```

- [ ] **Step 2: Update workspace**

```toml
# Cargo.toml
[workspace]
resolver = "2"
members = [
    "crates/eci-core",
    "crates/eci-github",
    "crates/eci-cli",
]
```

- [ ] **Step 3: Implement GitHub client**

```rust
// crates/eci-github/src/lib.rs
use eci_core::config::Config;
use eci_core::error::{EciError, Result};
use octocrab::Octocrab;
use std::path::PathBuf;

pub struct GitHubClient {
    octocrab: Octocrab,
}

#[derive(Debug)]
pub struct RepoInfo {
    pub full_name: String,
    pub name: String,
    pub description: Option<String>,
    pub default_branch: String,
    pub clone_url: String,
}

impl GitHubClient {
    pub async fn new(config: &Config) -> Result<Self> {
        let octocrab = Octocrab::builder()
            .personal_token(config.github.token.clone())
            .build()
            .map_err(|e| EciError::GitHub(format!("Failed to create client: {}", e)))?;
        Ok(Self { octocrab })
    }

    pub async fn list_repos(&self, owner: &str) -> Result<Vec<RepoInfo>> {
        let repos = self
            .octocrab
            .repos()
            .list_for_user(owner)
            .send()
            .await
            .map_err(|e| EciError::GitHub(format!("Failed to list repos: {}", e)))?;

        Ok(repos
            .items
            .into_iter()
            .map(|r| RepoInfo {
                full_name: r.full_name.unwrap_or_default(),
                name: r.name,
                description: r.description,
                default_branch: r.default_branch.unwrap_or_else(|| "main".to_string()),
                clone_url: r.clone_url.unwrap_or_default(),
            })
            .collect())
    }

    pub fn clone_repo(clone_url: &str, dest: &PathBuf, token: &str) -> Result<()> {
        let url_with_token = clone_url
            .replacen("https://", &format!("https://x:{}@", token), 1);

        let _ = std::fs::remove_dir_all(dest);
        git2::Repository::clone(&url_with_token, dest)
            .map_err(|e| EciError::GitHub(format!("Failed to clone repo: {}", e)))?;
        Ok(())
    }
}
```

- [ ] **Step 4: Verify compilation**

Run: `cargo build`
Expected: Compiles successfully

- [ ] **Step 5: Commit**

```bash
git add crates/eci-github/ Cargo.toml
git commit -m "feat(github): add GitHub API client for repo listing and cloning"
```

---

## Phase 3: Docker Management

### Task 3.1: Docker Client

**Files:**
- Create: `crates/eci-docker/Cargo.toml`
- Create: `crates/eci-docker/src/lib.rs`
- Modify: `Cargo.toml` (add to workspace)

**Interfaces:**
- Produces: `DockerClient::new()`, `DockerClient::build_image()`, `DockerClient::run_container()`, `DockerClient::stop_container()`, `DockerClient::logs()`, `DockerClient::list_containers()`

- [ ] **Step 1: Create eci-docker crate**

```toml
# crates/eci-docker/Cargo.toml
[package]
name = "eci-docker"
version = "0.1.0"
edition = "2021"

[dependencies]
eci-core = { path = "../eci-core" }
bollard = "0.16"
tokio = { version = "1", features = ["full"] }
futures-util = "0.3"
tar = "0.4"
```

- [ ] **Step 2: Update workspace**

```toml
# Cargo.toml
[workspace]
resolver = "2"
members = [
    "crates/eci-core",
    "crates/eci-github",
    "crates/eci-docker",
    "crates/eci-cli",
]
```

- [ ] **Step 3: Implement Docker client**

```rust
// crates/eci-docker/src/lib.rs
use bollard::container::{ListContainersOptions, LogsOptions, RemoveContainerOptions, StopContainerOptions};
use bollard::image::BuildImageOptions;
use bollard::models::{ContainerSummary, CreateImageInfo};
use bollard::Docker;
use eci_core::error::{EciError, Result};
use eci_core::types::AppStatus;
use futures_util::stream::TryStreamExt;
use std::collections::HashMap;
use std::path::Path;
use tar::Builder as TarBuilder;

pub struct DockerClient {
    docker: Docker,
}

#[derive(Debug)]
pub struct ContainerInfo {
    pub id: String,
    pub name: String,
    pub image: String,
    pub status: AppStatus,
    pub ports: Vec<String>,
}

impl DockerClient {
    pub async fn new() -> Result<Self> {
        let docker = Docker::connect_with_local_defaults()
            .map_err(|e| EciError::Docker(format!("Failed to connect: {}", e)))?;
        Ok(Self { docker })
    }

    pub async fn build_image(
        &self,
        app_name: &str,
        dockerfile_path: &Path,
    ) -> Result<String> {
        let context_path = dockerfile_path
            .parent()
            .ok_or_else(|| EciError::Docker("Invalid Dockerfile path".into()))?;

        let tar_path = format!("/tmp/{}.tar", app_name);
        let tar_file = std::fs::File::create(&tar_path)?;
        let mut tar = TarBuilder::new(tar_file);
        tar.append_dir_all(".", context_path)?;
        tar.finish()?;

        let mut build_opts = BuildImageOptions::default();
        build_opts.dockerfile = "Dockerfile";
        build_opts.t = app_name;
        build_opts.rm = true;

        let tar_bytes = std::fs::read(&tar_path)?;
        let mut stream = self
            .docker
            .build_image(build_opts, None, Some(tar_bytes.into()));

        let mut image_id = String::new();
        while let Some(msg) = stream
            .try_next()
            .await
            .map_err(|e| EciError::Docker(format!("Build error: {}", e)))?
        {
            if let Some(id) = msg.id {
                image_id = id;
            }
        }

        let _ = std::fs::remove_file(&tar_path);
        Ok(image_id)
    }

    pub async fn run_container(
        &self,
        app_name: &str,
        image: &str,
        port: Option<u16>,
    ) -> Result<String> {
        use bollard::container::CreateContainerOptions;
        use bollard::models::{HostConfig, PortBinding};

        let mut port_bindings = HashMap::new();
        if let Some(p) = port {
            let binding = PortBinding {
                host_ip: Some("0.0.0.0".to_string()),
                host_port: Some(p.to_string()),
            };
            port_bindings.insert("80/tcp".to_string(), Some(vec![binding]));
        }

        let options = CreateContainerOptions {
            name: app_name.to_string(),
            ..Default::default()
        };

        let config = bollard::container::Config {
            image: Some(image.to_string()),
            host_config: Some(HostConfig {
                port_bindings: Some(port_bindings),
                ..Default::default()
            }),
            ..Default::default()
        };

        let info = self
            .docker
            .create_container(Some(options), config)
            .await
            .map_err(|e| EciError::Docker(format!("Create container error: {}", e)))?;

        self.docker
            .start_container::<String>(&info.id, None)
            .await
            .map_err(|e| EciError::Docker(format!("Start container error: {}", e)))?;

        Ok(info.id)
    }

    pub async fn stop_container(&self, container_id: &str) -> Result<()> {
        self.docker
            .stop_container(container_id, Some(StopContainerOptions { t: 10 }))
            .await
            .map_err(|e| EciError::Docker(format!("Stop error: {}", e)))?;
        Ok(())
    }

    pub async fn remove_container(&self, container_id: &str) -> Result<()> {
        self.docker
            .remove_container(
                container_id,
                Some(RemoveContainerOptions {
                    force: true,
                    ..Default::default()
                }),
            )
            .await
            .map_err(|e| EciError::Docker(format!("Remove error: {}", e)))?;
        Ok(())
    }

    pub async fn logs(&self, container_id: &str) -> Result<Vec<String>> {
        let options = LogsOptions {
            stdout: true,
            stderr: true,
            tail: "100".to_string(),
            ..Default::default()
        };

        let mut logs = Vec::new();
        let mut stream = self.docker.logs(container_id, Some(options));

        while let Some(line) = stream
            .try_next()
            .await
            .map_err(|e| EciError::Docker(format!("Logs error: {}", e)))?
        {
            logs.push(line.to_string());
        }

        Ok(logs)
    }

    pub async fn list_containers(&self) -> Result<Vec<ContainerInfo>> {
        let options = ListContainersOptions::<String> {
            all: true,
            ..Default::default()
        };

        let containers = self
            .docker
            .list_containers(Some(options))
            .await
            .map_err(|e| EciError::Docker(format!("List error: {}", e)))?;

        Ok(containers
            .into_iter()
            .map(|c| {
                let name = c
                    .names
                    .and_then(|n| n.first().cloned())
                    .unwrap_or_default()
                    .trim_start_matches('/')
                    .to_string();

                let status = match c.state.as_deref() {
                    Some("running") => AppStatus::Running,
                    Some("exited") => AppStatus::Stopped,
                    _ => AppStatus::Stopped,
                };

                ContainerInfo {
                    id: c.id.unwrap_or_default(),
                    name,
                    image: c.image.unwrap_or_default(),
                    status,
                    ports: c
                        .ports
                        .map(|ps| {
                            ps.iter()
                                .filter_map(|p| {
                                    p.public_port
                                        .map(|pp| format!("{}:{}", pp, p.private_port))
                                })
                                .collect()
                        })
                        .unwrap_or_default(),
                }
            })
            .collect())
    }
}
```

- [ ] **Step 4: Verify compilation**

Run: `cargo build`
Expected: Compiles successfully

- [ ] **Step 5: Commit**

```bash
git add crates/eci-docker/ Cargo.toml
git commit -m "feat(docker): add Docker client for build, run, logs, rollback"
```

---

## Phase 4: Database Provisioning

### Task 4.1: Database Provisioner

**Files:**
- Create: `crates/eci-db/Cargo.toml`
- Create: `crates/eci-db/src/lib.rs`
- Modify: `Cargo.toml` (add to workspace)

**Interfaces:**
- Produces: `DbProvisioner::provision()`, `DbProvisioner::generate_credentials()`, `DbProvisioner::get_connection_string()`

- [ ] **Step 1: Create eci-db crate**

```toml
# crates/eci-db/Cargo.toml
[package]
name = "eci-db"
version = "0.1.0"
edition = "2021"

[dependencies]
eci-core = { path = "../eci-core" }
eci-docker = { path = "../eci-docker" }
rand = "0.8"
```

- [ ] **Step 2: Update workspace**

```toml
# Cargo.toml
[workspace]
resolver = "2"
members = [
    "crates/eci-core",
    "crates/eci-github",
    "crates/eci-docker",
    "crates/eci-db",
    "crates/eci-cli",
]
```

- [ ] **Step 3: Implement database provisioner**

```rust
// crates/eci-db/src/lib.rs
use eci_core::config::Config;
use eci_core::error::{EciError, Result};
use eci_core::types::DbInfo;
use eci_docker::DockerClient;
use rand::Rng;
use std::fs;
use std::path::PathBuf;

pub struct DbProvisioner<'a> {
    docker: &'a DockerClient,
    config: &'a Config,
}

#[derive(Debug, Clone)]
pub enum DbType {
    Postgres,
    Mongo,
    Redis,
    MySQL,
}

impl DbType {
    pub fn image(&self) -> &str {
        match self {
            DbType::Postgres => "postgres:16-alpine",
            DbType::Mongo => "mongo:7",
            DbType::Redis => "redis:7-alpine",
            DbType::MySQL => "mysql:8",
        }
    }

    pub fn default_port(&self) -> u16 {
        match self {
            DbType::Postgres => 5432,
            DbType::Mongo => 27017,
            DbType::Redis => 6379,
            DbType::MySQL => 3306,
        }
    }

    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "postgres" | "postgresql" => Ok(DbType::Postgres),
            "mongo" | "mongodb" => Ok(DbType::Mongo),
            "redis" => Ok(DbType::Redis),
            "mysql" => Ok(DbType::MySQL),
            _ => Err(EciError::Database(format!("Unsupported DB type: {}", s))),
        }
    }
}

impl<'a> DbProvisioner<'a> {
    pub fn new(docker: &'a DockerClient, config: &'a Config) -> Self {
        Self { docker, config }
    }

    pub fn generate_credentials(db_type: &DbType, app_name: &str) -> Result<(String, String)> {
        let mut rng = rand::thread_rng();
        let password: String = (0..24)
            .map(|_| {
                let idx = rng.gen_range(0..36);
                if idx < 10 {
                    (b'0' + idx) as char
                } else {
                    (b'a' + idx - 10) as char
                }
            })
            .collect();

        let username = match db_type {
            DbType::Postgres => "postgres".to_string(),
            DbType::Mongo => "admin".to_string(),
            DbType::Redis => "default".to_string(),
            DbType::MySQL => "root".to_string(),
        };

        Ok((username, password))
    }

    pub async fn provision(
        &self,
        app_name: &str,
        db_type: &DbType,
    ) -> Result<DbInfo> {
        let (username, password) = Self::generate_credentials(db_type, app_name)?;

        let secrets_dir = Config::config_dir()?.join("secrets");
        fs::create_dir_all(&secrets_dir)?;
        let env_path = secrets_dir.join(format!("{}.env", app_name));

        let env_content = match db_type {
            DbType::Postgres => format!(
                "POSTGRES_USER={}\nPOSTGRES_PASSWORD={}\nPOSTGRES_DB={}",
                username, password, app_name
            ),
            DbType::Mongo => format!(
                "MONGO_INITDB_ROOT_USERNAME={}\nMONGO_INITDB_ROOT_PASSWORD={}",
                username, password
            ),
            DbType::Redis => format!("REDIS_PASSWORD={}", password),
            DbType::MySQL => format!(
                "MYSQL_ROOT_PASSWORD={}\nMYSQL_DATABASE={}",
                password, app_name
            ),
        };

        fs::write(&env_path, env_content)?;

        let container_name = format!("eci-{}", app_name);
        let image = db_type.image();

        self.docker
            .run_container(&container_name, image, Some(db_type.default_port()))
            .await?;

        let connection_string = match db_type {
            DbType::Postgres => format!(
                "postgresql://{}:{}@localhost:{}/{}",
                username, password, db_type.default_port(), app_name
            ),
            DbType::Mongo => format!(
                "mongodb://{}:{}@localhost:{}/{}",
                username, password, db_type.default_port(), app_name
            ),
            DbType::Redis => format!(
                "redis://:{}@localhost:{}",
                password, db_type.default_port()
            ),
            DbType::MySQL => format!(
                "mysql://{}:{}@localhost:{}/{}",
                username, password, db_type.default_port(), app_name
            ),
        };

        Ok(DbInfo {
            app_name: app_name.to_string(),
            db_type: format!("{:?}", db_type),
            connection_string,
            credentials_path: env_path.to_string_lossy().to_string(),
        })
    }
}
```

- [ ] **Step 4: Verify compilation**

Run: `cargo build`
Expected: Compiles successfully

- [ ] **Step 5: Commit**

```bash
git add crates/eci-db/ Cargo.toml
git commit -m "feat(db): add database provisioner for Postgres, Mongo, Redis, MySQL"
```

---

## Phase 5: Deploy Orchestration

### Task 5.1: Deploy Engine

**Files:**
- Create: `crates/eci-deploy/Cargo.toml`
- Create: `crates/eci-deploy/src/lib.rs`
- Modify: `Cargo.toml` (add to workspace)

**Interfaces:**
- Produces: `DeployEngine::deploy()`, `DeployEngine::rollback()`, `DeployEngine::health_check()`

- [ ] **Step 1: Create eci-deploy crate**

```toml
# crates/eci-deploy/Cargo.toml
[package]
name = "eci-deploy"
version = "0.1.0"
edition = "2021"

[dependencies]
eci-core = { path = "../eci-core" }
eci-github = { path = "../eci-github" }
eci-docker = { path = "../eci-docker" }
eci-db = { path = "../eci-db" }
tokio = { version = "1", features = ["full"] }
```

- [ ] **Step 2: Update workspace**

```toml
# Cargo.toml
[workspace]
resolver = "2"
members = [
    "crates/eci-core",
    "crates/eci-github",
    "crates/eci-docker",
    "crates/eci-db",
    "crates/eci-deploy",
    "crates/eci-cli",
]
```

- [ ] **Step 3: Implement deploy engine**

```rust
// crates/eci-deploy/src/lib.rs
use eci_core::config::Config;
use eci_core::error::{EciError, Result};
use eci_core::state::State;
use eci_core::types::{App, AppStatus, Deployment};
use eci_docker::DockerClient;
use eci_github::GitHubClient;
use std::path::PathBuf;
use std::time::Duration;

pub struct DeployEngine<'a> {
    docker: &'a DockerClient,
    github: &'a GitHubClient,
    state: &'a State,
    config: &'a Config,
}

pub struct DeployResult {
    pub app: App,
    pub db_info: Option<eci_core::types::DbInfo>,
}

impl<'a> DeployEngine<'a> {
    pub fn new(
        docker: &'a DockerClient,
        github: &'a GitHubClient,
        state: &'a State,
        config: &'a Config,
    ) -> Self {
        Self {
            docker,
            github,
            state,
            config,
        }
    }

    pub async fn deploy(
        &self,
        repo: &str,
        app_name: &str,
        project_name: &str,
        description: Option<&str>,
        db_type: Option<&str>,
        port: Option<u16>,
    ) -> Result<DeployResult> {
        let app_dir = PathBuf::from(format!("/tmp/eci-{}", app_name));

        println!("Cloning {}...", repo);
        GitHubClient::clone_repo(
            &format!("https://github.com/{}", repo),
            &app_dir,
            &self.config.github.token,
        )?;

        println!("Building image...");
        let image_tag = format!("{}:latest", app_name);
        self.docker
            .build_image(app_name, &app_dir.join("Dockerfile"))
            .await?;

        let app = self
            .state
            .create_app(app_name, project_name, repo, description, &image_tag)?;

        println!("Starting container...");
        let container_id = self.docker.run_container(app_name, &image_tag, port).await?;

        self.state.update_app_status(app_name, &AppStatus::Running)?;

        let mut db_info = None;
        if let Some(db_type_str) = db_type {
            println!("Provisioning database...");
            let db_type = eci_db::DbType::from_str(db_type_str)?;
            let provisioner = eci_db::DbProvisioner::new(self.docker, self.config);
            db_info = Some(provisioner.provision(app_name, &db_type).await?);
        }

        println!("Health checking...");
        let healthy = self.health_check(port.unwrap_or(80)).await;
        if !healthy {
            self.state.update_app_status(app_name, &AppStatus::Unhealthy)?;
        }

        println!("Deploy complete!");
        let mut updated_app = app;
        updated_app.container_id = Some(container_id);
        updated_app.status = if healthy {
            AppStatus::Running
        } else {
            AppStatus::Unhealthy
        };

        Ok(DeployResult {
            app: updated_app,
            db_info,
        })
    }

    pub async fn rollback(&self, app_name: &str) -> Result<()> {
        let app = self
            .state
            .get_app(app_name)?
            .ok_or_else(|| EciError::Deploy(format!("App '{}' not found", app_name)))?;

        if let Some(container_id) = &app.container_id {
            println!("Stopping current container...");
            self.docker.stop_container(container_id).await?;
        }

        println!("Rolling back to previous version...");
        let old_image = format!("{}:previous", app_name);
        self.docker
            .run_container(app_name, &old_image, app.port)
            .await?;

        self.state.update_app_status(app_name, &AppStatus::Running)?;
        println!("Rollback complete!");
        Ok(())
    }

    pub async fn health_check(&self, port: u16) -> bool {
        let timeout = self.config.deploy.health_check_timeout_secs;
        let start = std::time::Instant::now();

        while start.elapsed() < Duration::from_secs(timeout) {
            if let Ok(resp) = reqwest::get(format!("http://localhost:{}", port)).await {
                if resp.status().is_success() {
                    return true;
                }
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
        false
    }
}
```

- [ ] **Step 4: Verify compilation**

Run: `cargo build`
Expected: Compiles successfully

- [ ] **Step 5: Commit**

```bash
git add crates/eci-deploy/ Cargo.toml
git commit -m "feat(deploy): add deployment orchestration with health checks"
```

---

### Task 5.2: Auto-Deploy Polling

**Files:**
- Modify: `crates/eci-deploy/src/lib.rs` (add Poller struct)
- Modify: `crates/eci-deploy/Cargo.toml` (add octocrab)

**Interfaces:**
- Produces: `Poller::new()`, `Poller::start()`, `Poller::stop()`

- [ ] **Step 1: Add octocrab dependency**

```toml
# crates/eci-deploy/Cargo.toml (add to [dependencies])
octocrab = "0.39"
```

- [ ] **Step 2: Add Poller to deploy crate**

Append to `crates/eci-deploy/src/lib.rs`:

```rust
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct Poller {
    running: Arc<AtomicBool>,
}

impl Poller {
    pub fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    pub async fn start(
        &self,
        app_name: &str,
        repo: &str,
        branch: &str,
        config: Config,
        state: State,
        docker: DockerClient,
    ) -> Result<()> {
        self.running.store(true, Ordering::SeqCst);
        let running = self.running.clone();
        let app_name = app_name.to_string();
        let repo = repo.to_string();
        let branch = branch.to_string();

        tokio::spawn(async move {
            let github = GitHubClient::new(&config).await.ok();
            let mut last_sha = String::new();

            while running.load(Ordering::SeqCst) {
                if let Some(gh) = &github {
                    if let Ok(repos) = gh.list_repos(&repo).await {
                        if let Some(r) = repos.first() {
                            let current_sha = r.default_branch.clone();
                            if !last_sha.is_empty() && current_sha != last_sha {
                                println!("New commit detected, deploying {}...", app_name);
                                let deploy_engine = DeployEngine::new(&docker, gh, &state, &config);
                                let _ = deploy_engine
                                    .deploy(&repo, &app_name, "default", None, None, None)
                                    .await;
                            }
                            last_sha = current_sha;
                        }
                    }
                }
                tokio::time::sleep(Duration::from_secs(config.deploy.poll_interval_secs)).await;
            }
        });

        Ok(())
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }
}
```

- [ ] **Step 3: Verify compilation**

Run: `cargo build`
Expected: Compiles successfully

- [ ] **Step 4: Commit**

```bash
git add crates/eci-deploy/
git commit -m "feat(deploy): add auto-deploy polling with GitHub API"
```

---

## Phase 6: CLI Commands

### Task 6.1: Full CLI Implementation

**Files:**
- Modify: `crates/eci-cli/Cargo.toml` (add dependencies)
- Modify: `crates/eci-cli/src/main.rs` (full implementation)

**Interfaces:**
- Consumes: all crate APIs
- Produces: complete `eci` binary

- [ ] **Step 1: Update eci-cli dependencies**

```toml
# crates/eci-cli/Cargo.toml
[package]
name = "eci-cli"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "eci"
path = "src/main.rs"

[dependencies]
eci-core = { path = "../eci-core" }
eci-github = { path = "../eci-github" }
eci-docker = { path = "../eci-docker" }
eci-db = { path = "../eci-db" }
eci-deploy = { path = "../eci-deploy" }
clap = { version = "4", features = ["derive"] }
tokio = { version = "1", features = ["full"] }
dialoguer = { version = "0.11", features = ["password"] }
console = "0.15"
```

- [ ] **Step 2: Implement full CLI**

```rust
// crates/eci-cli/src/main.rs
use clap::{Parser, Subcommand};
use console::Style;
use dialoguer::{Input, Select};

#[derive(Parser)]
#[command(name = "eci", about = "Internal CI/CD tool", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Init,
    Project {
        #[command(subcommand)]
        action: ProjectAction,
    },
    Deploy {
        repo: String,
        #[arg(short, long)]
        db: Option<String>,
        #[arg(long)]
        db_only: bool,
        #[arg(short, long)]
        port: Option<u16>,
        #[arg(long)]
        watch: bool,
    },
    Apps,
    Logs {
        app_name: String,
        #[arg(short, long, default_value = "100")]
        lines: usize,
    },
    Rollback {
        app_name: String,
    },
    Stop {
        app_name: String,
    },
    Start {
        app_name: String,
    },
    Remove {
        app_name: String,
    },
    Status,
    Dashboard,
}

#[derive(Subcommand)]
enum ProjectAction {
    Create,
    List,
    Delete { name: String },
}

#[tokio::main]
async fn main() -> eci_core::error::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init => cmd_init().await,
        Commands::Project { action } => match action {
            ProjectAction::Create => cmd_project_create().await,
            ProjectAction::List => cmd_project_list().await,
            ProjectAction::Delete { name } => cmd_project_delete(&name).await,
        },
        Commands::Deploy {
            repo,
            db,
            db_only,
            port,
            watch,
        } => cmd_deploy(&repo, db.as_deref(), db_only, port, watch).await,
        Commands::Apps => cmd_apps().await,
        Commands::Logs { app_name, lines } => cmd_logs(&app_name, lines).await,
        Commands::Rollback { app_name } => cmd_rollback(&app_name).await,
        Commands::Stop { app_name } => cmd_stop(&app_name).await,
        Commands::Start { app_name } => cmd_start(&app_name).await,
        Commands::Remove { app_name } => cmd_remove(&app_name).await,
        Commands::Status => cmd_status().await,
        Commands::Dashboard => {
            println!("Dashboard coming soon!");
            Ok(())
        }
    }
}

async fn cmd_init() -> eci_core::error::Result<()> {
    let title = Style::new().bold().dim();
    println!("{}", title.apply_to("easy-ci initialization"));

    let token: String = Input::new()
        .with_prompt("GitHub token")
        .interact_text()?;

    let org: String = Input::new()
        .with_prompt("Default org (optional)")
        .default(String::new())
        .interact_text()?;

    let mut config = eci_core::config::Config::load()?;
    config.github.token = token;
    config.github.default_org = if org.is_empty() { None } else { Some(org) };
    config.save()?;

    println!("Config saved to ~/.eci/config.toml");
    Ok(())
}

async fn cmd_project_create() -> eci_core::error::Result<()> {
    let state = eci_core::state::State::new()?;

    let name: String = Input::new()
        .with_prompt("Project name")
        .interact_text()?;

    let description: String = Input::new()
        .with_prompt("Description (optional)")
        .default(String::new())
        .interact_text()?;

    let desc = if description.is_empty() {
        None
    } else {
        Some(description.as_str())
    };

    state.create_project(&name, desc)?;
    println!("Project '{}' created!", name);
    Ok(())
}

async fn cmd_project_list() -> eci_core::error::Result<()> {
    let state = eci_core::state::State::new()?;
    let projects = state.list_projects()?;

    if projects.is_empty() {
        println!("No projects. Create one with: eci project create");
        return Ok(());
    }

    let header = Style::new().bold();
    println!("{}", header.apply_to("Projects:"));
    for p in &projects {
        println!("  {} - {}", p.name, p.description.as_deref().unwrap_or(""));
    }
    Ok(())
}

async fn cmd_project_delete(name: &str) -> eci_core::error::Result<()> {
    let state = eci_core::state::State::new()?;
    if state.delete_project(name)? {
        println!("Project '{}' deleted!", name);
    } else {
        println!("Project '{}' not found.", name);
    }
    Ok(())
}

async fn cmd_deploy(
    repo: &str,
    db: Option<&str>,
    db_only: bool,
    port: Option<u16>,
    _watch: bool,
) -> eci_core::error::Result<()> {
    let config = eci_core::config::Config::load()?;
    let state = eci_core::state::State::new()?;
    let docker = eci_docker::DockerClient::new().await?;
    let github = eci_github::GitHubClient::new(&config).await?;

    let projects = state.list_projects()?;
    if projects.is_empty() {
        println!("No projects. Create one first: eci project create");
        return Ok(());
    }

    let project_names: Vec<&str> = projects.iter().map(|p| p.name.as_str()).collect();
    let project_idx = if projects.len() == 1 {
        0
    } else {
        Select::new()
            .with_prompt("Select project")
            .items(&project_names)
            .default(0)
            .interact()?
    };

    let app_name: String = Input::new()
        .with_prompt("App name (unique)")
        .interact_text()?;

    let description: String = Input::new()
        .with_prompt("Description (optional)")
        .default(String::new())
        .interact_text()?;

    let desc = if description.is_empty() {
        None
    } else {
        Some(description.as_str())
    };

    let deploy_engine = eci_deploy::DeployEngine::new(&docker, &github, &state, &config);
    let result = deploy_engine
        .deploy(repo, &app_name, &project_names[project_idx], desc, db, port)
        .await?;

    println!("Deployed {} successfully!", app_name);
    if let Some(db_info) = &result.db_info {
        println!("DB connection: {}", db_info.connection_string);
    }
    Ok(())
}

async fn cmd_apps() -> eci_core::error::Result<()> {
    let state = eci_core::state::State::new()?;
    let apps = state.list_apps()?;

    if apps.is_empty() {
        println!("No apps deployed. Deploy with: eci deploy <repo>");
        return Ok(());
    }

    let header = Style::new().bold();
    println!(
        "{}",
        header.apply_to(format!("{:<20} {:<12} {:<20} {:<10}", "NAME", "STATUS", "IMAGE", "PROJECT"))
    );
    for app in &apps {
        let status_icon = match app.status {
            eci_core::types::AppStatus::Running => "●",
            eci_core::types::AppStatus::Stopped => "○",
            eci_core::types::AppStatus::Unhealthy => "◐",
            eci_core::types::AppStatus::Deploying => "◑",
        };
        println!(
            "{:<20} {} {:<10} {:<20} {:<10}",
            app.name, status_icon, format!("{:?}", app.status), app.image_tag, app.project_name
        );
    }
    Ok(())
}

async fn cmd_logs(app_name: &str, lines: usize) -> eci_core::error::Result<()> {
    let state = eci_core::state::State::new()?;
    let app = state
        .get_app(app_name)?
        .ok_or_else(|| eci_core::error::EciError::Deploy(format!("App '{}' not found", app_name)))?;

    let container_id = app
        .container_id
        .ok_or_else(|| eci_core::error::EciError::Deploy("No container running".into()))?;

    let docker = eci_docker::DockerClient::new().await?;
    let logs = docker.logs(&container_id).await?;

    for line in logs.iter().take(lines) {
        print!("{}", line);
    }
    Ok(())
}

async fn cmd_rollback(app_name: &str) -> eci_core::error::Result<()> {
    let config = eci_core::config::Config::load()?;
    let state = eci_core::state::State::new()?;
    let docker = eci_docker::DockerClient::new().await?;
    let github = eci_github::GitHubClient::new(&config).await?;

    let deploy_engine = eci_deploy::DeployEngine::new(&docker, &github, &state, &config);
    deploy_engine.rollback(app_name).await?;
    Ok(())
}

async fn cmd_stop(app_name: &str) -> eci_core::error::Result<()> {
    let state = eci_core::state::State::new()?;
    let app = state
        .get_app(app_name)?
        .ok_or_else(|| eci_core::error::EciError::Deploy(format!("App '{}' not found", app_name)))?;

    if let Some(container_id) = &app.container_id {
        let docker = eci_docker::DockerClient::new().await?;
        docker.stop_container(container_id).await?;
        state.update_app_status(app_name, &eci_core::types::AppStatus::Stopped)?;
        println!("App '{}' stopped.", app_name);
    }
    Ok(())
}

async fn cmd_start(app_name: &str) -> eci_core::error::Result<()> {
    let state = eci_core::state::State::new()?;
    let app = state
        .get_app(app_name)?
        .ok_or_else(|| eci_core::error::EciError::Deploy(format!("App '{}' not found", app_name)))?;

    let docker = eci_docker::DockerClient::new().await?;
    let container_id = docker
        .run_container(app_name, &app.image_tag, app.port)
        .await?;
    state.update_app_status(app_name, &eci_core::types::AppStatus::Running)?;
    println!("App '{}' started.", app_name);
    Ok(())
}

async fn cmd_remove(app_name: &str) -> eci_core::error::Result<()> {
    let state = eci_core::state::State::new()?;
    let app = state
        .get_app(app_name)?
        .ok_or_else(|| eci_core::error::EciError::Deploy(format!("App '{}' not found", app_name)))?;

    if let Some(container_id) = &app.container_id {
        let docker = eci_docker::DockerClient::new().await?;
        docker.remove_container(container_id).await?;
    }
    println!("App '{}' removed.", app_name);
    Ok(())
}

async fn cmd_status() -> eci_core::error::Result<()> {
    let state = eci_core::state::State::new()?;
    let projects = state.list_projects()?;
    let apps = state.list_apps()?;

    let header = Style::new().bold();
    println!("{}", header.apply_to("easy-ci Status"));
    println!("Projects: {}", projects.len());
    println!(
        "Apps: {} (running: {})",
        apps.len(),
        apps.iter()
            .filter(|a| a.status == eci_core::types::AppStatus::Running)
            .count()
    );
    Ok(())
}
```

- [ ] **Step 3: Verify compilation**

Run: `cargo build`
Expected: Compiles successfully

- [ ] **Step 4: Commit**

```bash
git add crates/eci-cli/
git commit -m "feat(cli): implement all CLI commands with interactive prompts"
```

---

## Phase 7: TUI Dashboard

### Task 7.1: TUI Dashboard

**Files:**
- Create: `crates/eci-tui/Cargo.toml`
- Create: `crates/eci-tui/src/lib.rs`
- Create: `crates/eci-tui/src/app.rs`
- Create: `crates/eci-tui/src/ui.rs`
- Modify: `Cargo.toml` (add to workspace)
- Modify: `crates/eci-cli/Cargo.toml` (add eci-tui dep)
- Modify: `crates/eci-cli/src/main.rs` (wire up dashboard)

**Interfaces:**
- Consumes: `State`, `DockerClient`
- Produces: `run_dashboard()`

- [ ] **Step 1: Create eci-tui crate**

```toml
# crates/eci-tui/Cargo.toml
[package]
name = "eci-tui"
version = "0.1.0"
edition = "2021"

[dependencies]
eci-core = { path = "../eci-core" }
eci-docker = { path = "../eci-docker" }
ratatui = "0.28"
crossterm = "0.28"
```

- [ ] **Step 2: Update workspace**

```toml
# Cargo.toml
[workspace]
resolver = "2"
members = [
    "crates/eci-core",
    "crates/eci-github",
    "crates/eci-docker",
    "crates/eci-db",
    "crates/eci-deploy",
    "crates/eci-tui",
    "crates/eci-cli",
]
```

- [ ] **Step 3: Implement TUI app state**

```rust
// crates/eci-tui/src/app.rs
use eci_core::state::State;
use eci_core::types::{App, AppStatus, Project};

#[derive(PartialEq)]
pub enum ActiveTab {
    Projects,
    Apps,
    Logs,
}

pub struct App {
    pub projects: Vec<Project>,
    pub apps: Vec<App>,
    pub selected_project: usize,
    pub selected_app: usize,
    pub active_tab: ActiveTab,
    pub logs: Vec<String>,
    pub should_quit: bool,
}

impl App {
    pub fn new(state: &State) -> eci_core::error::Result<Self> {
        let projects = state.list_projects()?;
        let apps = state.list_apps()?;
        Ok(Self {
            projects,
            apps,
            selected_project: 0,
            selected_app: 0,
            active_tab: ActiveTab::Projects,
            logs: Vec::new(),
            should_quit: false,
        })
    }

    pub fn next_project(&mut self) {
        if !self.projects.is_empty() {
            self.selected_project = (self.selected_project + 1) % self.projects.len();
        }
    }

    pub fn previous_project(&mut self) {
        if !self.projects.is_empty() {
            if self.selected_project == 0 {
                self.selected_project = self.projects.len() - 1;
            } else {
                self.selected_project -= 1;
            }
        }
    }

    pub fn next_app(&mut self) {
        if !self.apps.is_empty() {
            self.selected_app = (self.selected_app + 1) % self.apps.len();
        }
    }

    pub fn previous_app(&mut self) {
        if !self.apps.is_empty() {
            if self.selected_app == 0 {
                self.selected_app = self.apps.len() - 1;
            } else {
                self.selected_app -= 1;
            }
        }
    }
}
```

- [ ] **Step 4: Implement TUI rendering**

```rust
// crates/eci-tui/src/ui.rs
use crate::app::{ActiveTab, App};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Tabs};
use ratatui::Frame;

pub fn draw(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(frame.area());

    draw_header(frame, chunks[0], app);
    draw_content(frame, chunks[1], app);
    draw_footer(frame, chunks[2]);
}

fn draw_header(frame: &mut Frame, area: Rect, app: &App) {
    let titles = vec!["Projects", "Apps", "Logs"];
    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL).title("easy-ci"))
        .select(match app.active_tab {
            ActiveTab::Projects => 0,
            ActiveTab::Apps => 1,
            ActiveTab::Logs => 2,
        })
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
    frame.render_widget(tabs, area);
}

fn draw_content(frame: &mut Frame, area: Rect, app: &App) {
    match app.active_tab {
        ActiveTab::Projects => draw_projects(frame, area, app),
        ActiveTab::Apps => draw_apps(frame, area, app),
        ActiveTab::Logs => draw_logs(frame, area, app),
    }
}

fn draw_projects(frame: &mut Frame, area: Rect, app: &App) {
    let items: Vec<ListItem> = app
        .projects
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let style = if i == app.selected_project {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(Line::from(Span::styled(&p.name, style)))
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Projects"));
    frame.render_widget(list, area);
}

fn draw_apps(frame: &mut Frame, area: Rect, app: &App) {
    let header = Line::from(vec![
        Span::styled(
            format!("{:<20} {:<12} {:<20}", "NAME", "STATUS", "IMAGE"),
            Style::default().fg(Color::Yellow),
        ),
    ]);

    let rows: Vec<Line> = app
        .apps
        .iter()
        .enumerate()
        .map(|(i, a)| {
            let status_icon = match a.status {
                AppStatus::Running => "●",
                AppStatus::Stopped => "○",
                AppStatus::Unhealthy => "◐",
                AppStatus::Deploying => "◑",
            };
            let style = if i == app.selected_app {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            Line::from(Span::styled(
                format!(
                    "{:<20} {} {:<10} {:<20}",
                    a.name,
                    status_icon,
                    format!("{:?}", a.status),
                    a.image_tag
                ),
                style,
            ))
        })
        .collect();

    let mut lines = vec![header];
    lines.extend(rows);

    let paragraph = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title("Apps"));
    frame.render_widget(paragraph, area);
}

fn draw_logs(frame: &mut Frame, area: Rect, app: &App) {
    let text: Vec<Line> = app
        .logs
        .iter()
        .map(|l| Line::from(Span::raw(l)))
        .collect();

    let paragraph = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL).title("Logs"))
        .scroll((0, 0));
    frame.render_widget(paragraph, area);
}

fn draw_footer(frame: &mut Frame, area: Rect) {
    let footer = Line::from(vec![
        Span::styled(" F1:help ", Style::default().fg(Color::DarkGray)),
        Span::styled(" F2:projects ", Style::default().fg(Color::DarkGray)),
        Span::styled(" F3:apps ", Style::default().fg(Color::DarkGray)),
        Span::styled(" F4:logs ", Style::default().fg(Color::DarkGray)),
        Span::styled(" q:quit ", Style::default().fg(Color::DarkGray)),
    ]);
    let paragraph = Paragraph::new(footer)
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(paragraph, area);
}
```

- [ ] **Step 5: Implement TUI main loop**

```rust
// crates/eci-tui/src/lib.rs
pub mod app;
pub mod ui;

use crate::app::App;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use eci_core::state::State;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;

pub fn run_dashboard(state: &State) -> eci_core::error::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(state)?;

    loop {
        terminal.draw(|f| ui::draw(f, &app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Char('q') => {
                        app.should_quit = true;
                    }
                    KeyCode::F(2) => {
                        app.active_tab = crate::app::ActiveTab::Projects;
                    }
                    KeyCode::F(3) => {
                        app.active_tab = crate::app::ActiveTab::Apps;
                    }
                    KeyCode::F(4) => {
                        app.active_tab = crate::app::ActiveTab::Logs;
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        match app.active_tab {
                            crate::app::ActiveTab::Projects => app.previous_project(),
                            crate::app::ActiveTab::Apps => app.previous_app(),
                            _ => {}
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        match app.active_tab {
                            crate::app::ActiveTab::Projects => app.next_project(),
                            crate::app::ActiveTab::Apps => app.next_app(),
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}
```

- [ ] **Step 6: Wire up dashboard in CLI**

Add to `crates/eci-cli/Cargo.toml`:
```toml
eci-tui = { path = "../eci-tui" }
```

Update the dashboard command in `main.rs`:
```rust
Commands::Dashboard => {
    let state = eci_core::state::State::new()?;
    eci_tui::run_dashboard(&state)?;
    Ok(())
}
```

- [ ] **Step 7: Verify compilation**

Run: `cargo build`
Expected: Compiles successfully

- [ ] **Step 8: Commit**

```bash
git add crates/eci-tui/ crates/eci-cli/ Cargo.toml
git commit -m "feat(tui): add ratatui dashboard with project/app/log views"
```

---

## Final Verification

### Task 8.1: Full Build and Test

- [ ] **Step 1: Clean build**

Run: `cargo build --release`
Expected: Compiles with no errors

- [ ] **Step 2: Test basic commands**

Run: `./target/release/eci --help`
Expected: Shows all commands

Run: `./target/release/eci init`
Expected: Prompts for GitHub token

Run: `./target/release/eci project create`
Expected: Prompts for project name

- [ ] **Step 3: Commit final state**

```bash
git add .
git commit -m "chore: final build verification"
```

---

## Summary

| Phase | Tasks | Description |
|-------|-------|-------------|
| 1 | 1.1, 1.2 | Project scaffolding, core types, config, state |
| 2 | 2.1 | GitHub API client |
| 3 | 3.1 | Docker client |
| 4 | 4.1 | Database provisioner |
| 5 | 5.1, 5.2 | Deploy orchestration + auto-deploy polling |
| 6 | 6.1 | Full CLI implementation |
| 7 | 7.1 | TUI dashboard |
| 8 | 8.1 | Final verification |
