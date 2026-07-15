use clap::{Parser, Subcommand};
use colored::*;
use dialoguer::{Confirm, Input, Select};

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

#[derive(Parser)]
#[command(name = "eci", about = "easy-ci — internal deployment tool", version)]
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
        project: Option<String>,
        #[arg(short, long)]
        branch: Option<String>,
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
    Dashboard,
    Webhook {
        #[arg(short, long, default_value = "8080")]
        port: u16,
        #[arg(short, long)]
        secret: Option<String>,
    },
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
        Commands::Init => cmd_init()?,
        Commands::Project { action } => match action {
            ProjectAction::Create => cmd_project_create()?,
            ProjectAction::List => cmd_project_list()?,
            ProjectAction::Delete { name } => cmd_project_delete(&name)?,
        },
        Commands::Deploy { project, branch } => cmd_deploy(project, branch).await?,
        Commands::Apps => cmd_apps()?,
        Commands::Logs { name } => cmd_logs(&name).await?,
        Commands::Stop { name } => cmd_stop(&name).await?,
        Commands::Remove { name } => cmd_remove(&name).await?,
        Commands::Status { name } => cmd_status(&name)?,
        Commands::Dashboard => cmd_dashboard()?,
        Commands::Webhook { port, secret } => cmd_webhook(port, secret).await?,
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
            poll_interval_secs: 30,
            health_check_timeout_secs: 60,
            auto_rollback_on_unhealthy: true,
            auto_deploy_on_commit: true,
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
    project: Option<String>,
    branch: Option<String>,
) -> eci_core::error::Result<()> {
    print_banner();

    let config = eci_core::config::Config::load()?;
    let state = eci_core::state::State::new()?;
    let github = eci_github::GitHubClient::new(&config).await?;

    // Select or create project
    let project_name = match project {
        Some(p) => p,
        None => {
            let projects = state.list_projects()?;
            let choices: Vec<&str> = projects.iter().map(|p| p.name.as_str()).collect();

            if choices.is_empty() {
                println!("  {}", dim("No projects yet. Let's create one."));
                println!();
                let name: String = Input::with_theme(&dialoguer::theme::ColorfulTheme::default())
                    .with_prompt("Project name")
                    .interact()
                    .map_err(|e| {
                        eci_core::error::EciError::Config(format!("Input error: {}", e))
                    })?;
                state.create_project(&name, None)?;
                name
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

    // Fetch all repos with pagination
    println!();
    print!("  {}", dim("Fetching repositories..."));

    let all_repos = github.list_all_repos().await?;

    println!(" {}", success(format!("found {} repos", all_repos.len())));

    if all_repos.is_empty() {
        println!();
        println!(
            "  {}",
            dim("No repositories found. Check your GitHub token.")
        );
        return Ok(());
    }

    // Build repo choices with descriptions
    let repo_items: Vec<String> = all_repos
        .iter()
        .map(|r| {
            let desc = r.description.as_deref().unwrap_or("no description");
            let truncated = if desc.len() > 50 { &desc[..50] } else { desc };
            format!("{:<45} {}", r.full_name, truncated)
        })
        .collect();

    let repo_selection = Select::with_theme(&dialoguer::theme::ColorfulTheme::default())
        .with_prompt("Select repository")
        .items(&repo_items)
        .default(0)
        .interact()
        .map_err(|e| eci_core::error::EciError::Config(format!("Input error: {}", e)))?;

    let selected_repo = &all_repos[repo_selection];

    // Confirm
    println!();
    println!(
        "  {} Repository:  {}",
        brand("→"),
        bold_text(&selected_repo.full_name)
    );
    println!(
        "  {} Branch:      {}",
        brand("→"),
        bold_text(branch.as_deref().unwrap_or("main"))
    );
    println!();

    let confirmed = Confirm::with_theme(&dialoguer::theme::ColorfulTheme::default())
        .with_prompt("Deploy this repo?")
        .default(true)
        .interact()
        .map_err(|e| eci_core::error::EciError::Config(format!("Input error: {}", e)))?;

    if !confirmed {
        println!();
        println!("  {}", dim("Deploy cancelled."));
        return Ok(());
    }

    // App name
    let default_name = selected_repo.name.replace('_', "-");
    let app_name: String = Input::with_theme(&dialoguer::theme::ColorfulTheme::default())
        .with_prompt("App name")
        .default(default_name.clone())
        .interact_text()
        .map_err(|e| eci_core::error::EciError::Config(format!("Input error: {}", e)))?;

    println!();
    println!("  {}", dim("Starting deploy..."));
    println!();

    let docker = eci_docker::DockerClient::new().await?;
    let engine = eci_deploy::DeployEngine::new(&docker, &github, &state, &config);

    let result = engine
        .deploy(
            &selected_repo.full_name,
            &app_name,
            &project_name,
            selected_repo.description.as_deref(),
            None,
            None,
        )
        .await?;

    println!();
    println!("  {}  Deployed {}", success("✔"), bold_text(&app_name));
    if let Some(db) = &result.db_info {
        println!(
            "  {} Database: {} ({})",
            dim("→"),
            db.db_type,
            dim(&db.connection_string)
        );
    }
    println!();
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
    let state = eci_core::state::State::new()?;
    let app = state.get_app(app_name)?.ok_or_else(|| {
        eci_core::error::EciError::Deploy(format!("App '{}' not found", app_name))
    })?;

    let container_id = app
        .container_id
        .ok_or_else(|| eci_core::error::EciError::Deploy("No container running".into()))?;

    let docker = eci_docker::DockerClient::new().await?;
    let logs = docker.logs(&container_id).await?;

    for line in logs.iter().rev().take(50).rev() {
        println!("  {}", dim(line));
    }
    Ok(())
}

async fn cmd_stop(app_name: &str) -> eci_core::error::Result<()> {
    print_banner();
    let state = eci_core::state::State::new()?;
    let app = state.get_app(app_name)?.ok_or_else(|| {
        eci_core::error::EciError::Deploy(format!("App '{}' not found", app_name))
    })?;

    if let Some(cid) = &app.container_id {
        let docker = eci_docker::DockerClient::new().await?;
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

    let state = eci_core::state::State::new()?;
    let app = state.get_app(app_name)?.ok_or_else(|| {
        eci_core::error::EciError::Deploy(format!("App '{}' not found", app_name))
    })?;

    if let Some(cid) = &app.container_id {
        let docker = eci_docker::DockerClient::new().await?;
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

fn cmd_dashboard() -> eci_core::error::Result<()> {
    let state = eci_core::state::State::new()?;
    eci_tui::run_dashboard(&state)
}

async fn cmd_webhook(port: u16, secret: Option<String>) -> eci_core::error::Result<()> {
    print_banner();
    println!("  {}", header("Starting webhook server"));
    println!();

    let config = eci_core::config::Config::load()?;
    let state = eci_core::state::State::new()?;
    let docker = eci_docker::DockerClient::new().await?;

    let webhook_secret = secret.unwrap_or_else(|| {
        println!(
            "  {}",
            warning("No webhook secret provided. Using default.")
        );
        println!(
            "  {}",
            dim("Set --secret or ECI_WEBHOOK_SECRET env var for production.")
        );
        "eci-webhook-secret".to_string()
    });

    println!(
        "  {} Listening on port {}",
        brand("→"),
        bold_text(port.to_string())
    );
    println!(
        "  {} Webhook URL: http://localhost:{}/webhook",
        brand("→"),
        port
    );
    println!();
    println!(
        "  {} Register this URL in your GitHub repo Settings → Webhooks",
        dim("→")
    );
    println!("  {} Content type: application/json", dim("→"));
    println!("  {} Events: Just the push event", dim("→"));
    println!();
    println!("  {} Press Ctrl+C to stop", dim("→"));
    println!();

    eci_webhook::start_webhook_server(port, config, state, docker, webhook_secret).await
}
