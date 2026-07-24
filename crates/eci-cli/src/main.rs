use clap::{Parser, Subcommand};
use colored::*;
use dialoguer::{Confirm, Input, Select};
use tracing::{debug, info};

fn brand<S: fmt::Display>(s: S) -> ColoredString {
    s.to_string().truecolor(108, 92, 231).bold()
}
fn success<S: fmt::Display>(s: S) -> ColoredString {
    s.to_string().truecolor(0, 184, 148).bold()
}
fn warning<S: fmt::Display>(s: S) -> ColoredString {
    s.to_string().truecolor(253, 203, 110)
}
fn dim<S: fmt::Display>(s: S) -> ColoredString {
    s.to_string().truecolor(99, 110, 114)
}
fn bold_text<S: fmt::Display>(s: S) -> ColoredString {
    s.to_string().truecolor(223, 230, 233).bold()
}
fn header<S: fmt::Display>(s: S) -> ColoredString {
    s.to_string().truecolor(108, 92, 231).bold().underline()
}

use std::fmt;

#[derive(Parser, Debug)]
#[command(name = "eci", about = "easy-ci — internal deployment tool", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Init,
    Project {
        #[command(subcommand)]
        action: ProjectAction,
    },
    Deploy {
        repo: String,
        #[arg(short, long)]
        name: Option<String>,
        #[arg(long)]
        project: Option<String>,
        #[arg(short, long)]
        db: Option<String>,
        #[arg(long)]
        port: Option<u16>,
        #[arg(long)]
        watch: bool,
    },
    Apps,
    Logs {
        name: String,
    },
    Stop {
        name: String,
    },
    Remove {
        name: String,
    },
    Status {
        name: String,
    },
    History {
        name: String,
    },
    Rollback {
        name: String,
    },
    Dashboard,
}

#[derive(Subcommand, Debug)]
enum ProjectAction {
    Create,
    List,
    Delete { name: String },
}

#[tokio::main]
async fn main() -> eci_core::error::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .with_target(false)
        .init();

    let cli = Cli::parse();
    debug!(command = ?cli.command, "CLI command parsed");

    match cli.command {
        Commands::Init => cmd_init()?,
        Commands::Project { action } => match action {
            ProjectAction::Create => cmd_project_create()?,
            ProjectAction::List => cmd_project_list()?,
            ProjectAction::Delete { name } => cmd_project_delete(&name)?,
        },
        Commands::Deploy {
            repo,
            name,
            project,
            db,
            port,
            watch,
        } => cmd_deploy(&repo, name, project, db, port, watch).await?,
        Commands::Apps => cmd_apps()?,
        Commands::Logs { name } => cmd_logs(&name).await?,
        Commands::Stop { name } => cmd_stop(&name).await?,
        Commands::Remove { name } => cmd_remove(&name).await?,
        Commands::Status { name } => cmd_status(&name)?,
        Commands::History { name } => cmd_history(&name)?,
        Commands::Rollback { name } => cmd_rollback(&name).await?,
        Commands::Dashboard => cmd_dashboard()?,
    }

    Ok(())
}

fn print_banner() {
    println!();
    println!("  {}", brand("⚡ easy-ci"));
    println!();
}

fn cmd_init() -> eci_core::error::Result<()> {
    print_banner();

    let existing = eci_core::config::Config::load();
    if existing.is_ok() {
        println!("  {}  Already initialized", success("✔"));
        return Ok(());
    }

    println!("  {}", header("Let's set up easy-ci"));
    println!();

    let github_token: String = Input::with_theme(&dialoguer::theme::ColorfulTheme::default())
        .with_prompt("GitHub token")
        .interact()
        .map_err(|e| eci_core::error::EciError::Config(format!("Input error: {}", e)))?;

    let host: String = Input::with_theme(&dialoguer::theme::ColorfulTheme::default())
        .with_prompt("Docker host")
        .default("unix:///var/run/docker.sock".into())
        .interact_text()
        .map_err(|e| eci_core::error::EciError::Config(format!("Input error: {}", e)))?;

    let config = eci_core::config::Config {
        github: eci_core::config::GitHubConfig {
            token: github_token,
            default_org: None,
        },
        docker: eci_core::config::DockerConfig { host },
        deploy: eci_core::config::DeployConfig {
            health_check_timeout_secs: 60,
            auto_rollback_on_unhealthy: true,
        },
    };

    config.save()?;
    println!();
    println!("  {}  Config saved to ~/.eci/config.toml", success("✔"));
    println!();
    Ok(())
}

fn cmd_project_create() -> eci_core::error::Result<()> {
    print_banner();

    let name: String = Input::with_theme(&dialoguer::theme::ColorfulTheme::default())
        .with_prompt("Project name")
        .interact()
        .map_err(|e| eci_core::error::EciError::Config(format!("Input error: {}", e)))?;

    let desc: String = Input::with_theme(&dialoguer::theme::ColorfulTheme::default())
        .with_prompt("Description (optional)")
        .default("".into())
        .interact_text()
        .map_err(|e| eci_core::error::EciError::Config(format!("Input error: {}", e)))?;

    let state = eci_core::state::State::new()?;
    state.create_project(&name, if desc.is_empty() { None } else { Some(&desc) })?;

    println!();
    println!("  {}  Project {} created", success("✔"), bold_text(&name));
    println!();
    Ok(())
}

fn cmd_project_list() -> eci_core::error::Result<()> {
    print_banner();

    let state = eci_core::state::State::new()?;
    let projects = state.list_projects()?;

    if projects.is_empty() {
        println!("  {}", dim("No projects yet. Run: eci project create"));
        return Ok(());
    }

    println!("  {}", header(format!("Projects ({})", projects.len())));
    println!();
    for p in &projects {
        println!("    {} {}", success("●"), bold_text(&p.name));
        if let Some(desc) = &p.description {
            println!("      {}", dim(desc));
        }
    }
    println!();
    Ok(())
}

fn cmd_project_delete(name: &str) -> eci_core::error::Result<()> {
    print_banner();

    let confirmed = Confirm::with_theme(&dialoguer::theme::ColorfulTheme::default())
        .with_prompt(format!("Delete project '{}'?", name))
        .default(false)
        .interact()
        .map_err(|e| eci_core::error::EciError::Config(format!("Input error: {}", e)))?;

    if !confirmed {
        println!("  {}", dim("Cancelled"));
        return Ok(());
    }

    let state = eci_core::state::State::new()?;
    state.delete_project(name)?;
    println!();
    println!("  {}  Project {} deleted", success("✔"), bold_text(name));
    println!();
    Ok(())
}

async fn cmd_deploy(
    repo: &str,
    name: Option<String>,
    project: Option<String>,
    db: Option<String>,
    port: Option<u16>,
    watch: bool,
) -> eci_core::error::Result<()> {
    print_banner();

    let config = eci_core::config::Config::load()?;
    let state = eci_core::state::State::new()?;
    let github = eci_github::GitHubClient::new(&config).await?;

    // Resolve project name
    let project_name = match project {
        Some(p) => p,
        None => {
            let projects = state.list_projects()?;
            let choices: Vec<&str> = projects.iter().map(|p| p.name.as_str()).collect();

            if choices.is_empty() {
                println!("  {}", dim("No projects yet. Let's create one."));
                println!();
                let pname: String = Input::with_theme(&dialoguer::theme::ColorfulTheme::default())
                    .with_prompt("Project name")
                    .interact()
                    .map_err(|e| {
                        eci_core::error::EciError::Config(format!("Input error: {}", e))
                    })?;
                state.create_project(&pname, None)?;
                pname
            } else {
                let selection = Select::with_theme(&dialoguer::theme::ColorfulTheme::default())
                    .with_prompt("Select project")
                    .items(&choices)
                    .default(0)
                    .interact()
                    .map_err(|e| {
                        eci_core::error::EciError::Config(format!("Input error: {}", e))
                    })?;
                choices[selection].to_string()
            }
        }
    };

    // Resolve app name
    let app_name = match name {
        Some(n) => n,
        None => {
            let default_name = repo
                .split('/')
                .next_back()
                .unwrap_or(repo)
                .replace('_', "-");
            Input::with_theme(&dialoguer::theme::ColorfulTheme::default())
                .with_prompt("App name")
                .default(default_name)
                .interact_text()
                .map_err(|e| eci_core::error::EciError::Config(format!("Input error: {}", e)))?
        }
    };

    println!();
    println!(
        "  {} Deploying {} to project {}",
        brand("→"),
        bold_text(repo),
        bold_text(&project_name)
    );
    println!();
    println!("  {}", dim("Starting deploy..."));
    println!();

    let docker = eci_docker::DockerClient::new(&config.docker).await?;
    let engine = eci_deploy::DeployEngine::new(&docker, &github, &state, &config);

    let result = engine
        .deploy(repo, &app_name, &project_name, None, db.as_deref(), port)
        .await?;

    println!();
    println!("  {}  Deployed {}", success("✔"), bold_text(&app_name));
    if let Some(db_info) = &result.db_info {
        println!(
            "  {} Database: {} ({})",
            dim("→"),
            db_info.db_type,
            dim(&db_info.connection_string)
        );
    }
    println!();

    // Start polling if --watch
    if watch {
        println!("  {} Starting poller (60s interval)...", brand("→"));
        let poller = eci_deploy::Poller::new();
        poller
            .start(&app_name, repo, "main", config.clone(), state, docker)
            .await?;
        println!("  {} Press Ctrl+C to stop", dim("→"));
        // Keep running until interrupted
        tokio::signal::ctrl_c().await.ok();
        poller.stop();
    }

    Ok(())
}

fn cmd_apps() -> eci_core::error::Result<()> {
    print_banner();

    let state = eci_core::state::State::new()?;
    let apps = state.list_apps()?;

    if apps.is_empty() {
        println!("  {}", dim("No apps deployed yet. Run: eci deploy"));
        return Ok(());
    }

    println!("  {}", header(format!("Apps ({})", apps.len())));
    println!();
    for app in &apps {
        let icon = match app.status {
            eci_core::types::AppStatus::Running => success("●"),
            eci_core::types::AppStatus::Stopped => dim("○"),
            eci_core::types::AppStatus::Unhealthy => warning("◐"),
            eci_core::types::AppStatus::Deploying => brand("◑"),
        };

        println!(
            "    {} {} {}",
            icon,
            bold_text(&app.name),
            dim(&app.project_name)
        );
    }
    println!();
    Ok(())
}

async fn cmd_logs(app_name: &str) -> eci_core::error::Result<()> {
    print_banner();
    let config = eci_core::config::Config::load()?;
    let state = eci_core::state::State::new()?;
    let app = state.get_app(app_name)?.ok_or_else(|| {
        eci_core::error::EciError::Deploy(format!("App '{}' not found", app_name))
    })?;

    let container_id = app
        .container_id
        .ok_or_else(|| eci_core::error::EciError::Deploy("No container running".into()))?;

    let docker = eci_docker::DockerClient::new(&config.docker).await?;
    let logs = docker.logs(&container_id).await?;

    for line in logs.iter().rev().take(50).rev() {
        println!("  {}", dim(line));
    }
    Ok(())
}

async fn cmd_stop(app_name: &str) -> eci_core::error::Result<()> {
    print_banner();
    let config = eci_core::config::Config::load()?;
    let state = eci_core::state::State::new()?;
    let app = state.get_app(app_name)?.ok_or_else(|| {
        eci_core::error::EciError::Deploy(format!("App '{}' not found", app_name))
    })?;

    if let Some(cid) = &app.container_id {
        let docker = eci_docker::DockerClient::new(&config.docker).await?;
        docker.stop_container(cid).await?;
        println!("  {}  Stopped {}", success("✔"), bold_text(app_name));
    }
    Ok(())
}

async fn cmd_remove(app_name: &str) -> eci_core::error::Result<()> {
    print_banner();
    let confirmed = Confirm::with_theme(&dialoguer::theme::ColorfulTheme::default())
        .with_prompt(format!("Remove app '{}'?", app_name))
        .default(false)
        .interact()
        .map_err(|e| eci_core::error::EciError::Config(format!("Input error: {}", e)))?;

    if !confirmed {
        println!("  {}", dim("Cancelled"));
        return Ok(());
    }

    let config = eci_core::config::Config::load()?;
    let state = eci_core::state::State::new()?;
    let app = state.get_app(app_name)?.ok_or_else(|| {
        eci_core::error::EciError::Deploy(format!("App '{}' not found", app_name))
    })?;

    if let Some(cid) = &app.container_id {
        let docker = eci_docker::DockerClient::new(&config.docker).await?;
        docker.remove_container(cid).await?;
    }
    state.delete_app(app_name)?;
    println!("  {}  Removed {}", success("✔"), bold_text(app_name));
    Ok(())
}

fn cmd_status(app_name: &str) -> eci_core::error::Result<()> {
    print_banner();
    let state = eci_core::state::State::new()?;
    let app = state.get_app(app_name)?.ok_or_else(|| {
        eci_core::error::EciError::Deploy(format!("App '{}' not found", app_name))
    })?;

    println!("  {}", header(format!("Status: {}", app.name)));
    println!();

    let status_colored = match app.status {
        eci_core::types::AppStatus::Running => success("running"),
        eci_core::types::AppStatus::Stopped => dim("stopped"),
        eci_core::types::AppStatus::Unhealthy => warning("unhealthy"),
        eci_core::types::AppStatus::Deploying => brand("deploying"),
    };

    println!("    {} {}", bold_text("Status:"), status_colored);
    println!("    {} {}", bold_text("Image:"), dim(&app.image_tag));
    println!("    {} {}", bold_text("Project:"), dim(&app.project_name));
    if let Some(cid) = &app.container_id {
        let short = if cid.len() > 12 { &cid[..12] } else { cid };
        println!("    {} {}", bold_text("Container:"), dim(short));
    }
    println!();
    Ok(())
}

fn cmd_history(app_name: &str) -> eci_core::error::Result<()> {
    print_banner();
    let state = eci_core::state::State::new()?;
    let deployments = state.list_deployments(app_name)?;

    if deployments.is_empty() {
        println!("  {}", dim(format!("No deployments for '{}'.", app_name)));
        return Ok(());
    }

    println!(
        "  {}",
        header(format!("Deployments for {} (last 10)", app_name))
    );
    println!();
    for d in &deployments {
        println!(
            "    {} {} {} {}",
            dim(format!("#{}", d.id)),
            success(&d.version),
            dim(&d.image_tag),
            dim(d.created_at.format("%Y-%m-%d %H:%M").to_string())
        );
    }
    println!();
    Ok(())
}

async fn cmd_rollback(app_name: &str) -> eci_core::error::Result<()> {
    print_banner();
    info!(app = app_name, "Rolling back");

    let config = eci_core::config::Config::load()?;
    let state = eci_core::state::State::new()?;
    let github = eci_github::GitHubClient::new(&config).await?;
    let docker = eci_docker::DockerClient::new(&config.docker).await?;
    let engine = eci_deploy::DeployEngine::new(&docker, &github, &state, &config);

    engine.rollback(app_name).await?;

    println!();
    println!("  {}  Rolled back {}", success("✔"), bold_text(app_name));
    println!();
    Ok(())
}

fn cmd_dashboard() -> eci_core::error::Result<()> {
    let state = eci_core::state::State::new()?;
    eci_tui::run_dashboard(&state)
}
