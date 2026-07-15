use clap::{Parser, Subcommand};
use console::Style;
use dialoguer::{Confirm, Input, Select};

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
            db,
            db_only,
            port,
            watch,
        } => cmd_deploy(db.as_deref(), db_only, port, watch).await,
        Commands::Apps => cmd_apps().await,
        Commands::Logs { app_name, lines } => cmd_logs(&app_name, lines).await,
        Commands::Rollback { app_name } => cmd_rollback(&app_name).await,
        Commands::Stop { app_name } => cmd_stop(&app_name).await,
        Commands::Start { app_name } => cmd_start(&app_name).await,
        Commands::Remove { app_name } => cmd_remove(&app_name).await,
        Commands::Status => cmd_status().await,
        Commands::Dashboard => {
            let state = eci_core::state::State::new()?;
            eci_tui::run_dashboard(&state)?;
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

    let mut config = eci_core::config::Config::load()?;
    config.github.token = token;
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
    db: Option<&str>,
    _db_only: bool,
    port: Option<u16>,
    _watch: bool,
) -> eci_core::error::Result<()> {
    let config = eci_core::config::Config::load()?;
    let state = eci_core::state::State::new()?;

    // Step 1: Select or create project
    let projects = state.list_projects()?;
    let project_name = if projects.is_empty() {
        println!("No projects found. Let's create one first.");
        let name: String = Input::new()
            .with_prompt("Project name")
            .interact_text()?;
        let description: String = Input::new()
            .with_prompt("Description (optional)")
            .default(String::new())
            .interact_text()?;
        let desc = if description.is_empty() { None } else { Some(description.as_str()) };
        state.create_project(&name, desc)?;
        println!("Project '{}' created!", name);
        name
    } else {
        let project_names: Vec<&str> = projects.iter().map(|p| p.name.as_str()).collect();
        if projects.len() == 1 {
            println!("Using project: {}", project_names[0]);
            project_names[0].to_string()
        } else {
            let idx = Select::new()
                .with_prompt("Select project")
                .items(&project_names)
                .default(0)
                .interact()?;
            project_names[idx].to_string()
        }
    };

    // Step 2: Fetch repos and let user select
    let github = eci_github::GitHubClient::new(&config).await?;
    println!("Fetching repositories...");
    let repos = github.list_my_repos().await?;

    if repos.is_empty() {
        println!("No repositories found with this token.");
        return Ok(());
    }

    let repo_labels: Vec<String> = repos.iter().map(|r| {
        let desc = r.description.as_deref().unwrap_or("");
        if desc.is_empty() {
            r.full_name.clone()
        } else {
            format!("{} — {}", r.full_name, desc)
        }
    }).collect();

    let repo_idx = Select::new()
        .with_prompt("Select repository")
        .items(&repo_labels)
        .default(0)
        .interact()?;

    let selected_repo = &repos[repo_idx];
    println!("Selected: {}", selected_repo.full_name);

    // Step 3: Confirm
    if !Confirm::new()
        .with_prompt("Deploy this repository?")
        .default(true)
        .interact()?
    {
        println!("Deploy cancelled.");
        return Ok(());
    }

    // Step 4: App name and description
    let app_name: String = Input::new()
        .with_prompt("App name (unique)")
        .interact_text()?;

    let description: String = Input::new()
        .with_prompt("Description (optional)")
        .default(String::new())
        .interact_text()?;

    let desc = if description.is_empty() { None } else { Some(description.as_str()) };

    // Step 5: Deploy
    let docker = eci_docker::DockerClient::new().await?;
    let deploy_engine = eci_deploy::DeployEngine::new(&docker, &github, &state, &config);
    let result = deploy_engine
        .deploy(&selected_repo.full_name, &app_name, &project_name, desc, db, port)
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
    let _container_id = docker
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
