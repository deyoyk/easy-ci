# Task 1.1: Initialize Cargo Workspace

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
