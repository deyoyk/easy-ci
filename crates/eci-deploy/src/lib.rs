use eci_core::config::Config;
use eci_core::error::{EciError, Result};
use eci_core::state::State;
use eci_core::types::{App, AppStatus, DbInfo};
use eci_db::DbProvisioner;
use eci_docker::DockerClient;
use eci_github::GitHubClient;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

pub struct DeployEngine<'a> {
    docker: &'a DockerClient,
    github: &'a GitHubClient,
    state: &'a State,
    config: &'a Config,
}

pub struct DeployResult {
    pub app: App,
    pub db_info: Option<DbInfo>,
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

        let app =
            self.state
                .create_app(app_name, project_name, repo, description, &image_tag)?;

        println!("Starting container...");
        let container_id = self.docker.run_container(app_name, &image_tag, port).await?;

        self.state.update_app_status(app_name, &AppStatus::Running)?;

        let mut db_info = None;
        if let Some(db_type_str) = db_type {
            println!("Provisioning database...");
            let db_type = eci_db::DbType::from_str(db_type_str)?;
            let provisioner = DbProvisioner::new(self.docker, self.config);
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
        _branch: &str,
        config: Config,
        state: State,
        docker: DockerClient,
    ) -> Result<()> {
        self.running.store(true, Ordering::SeqCst);
        let running = self.running.clone();
        let app_name = app_name.to_string();
        let repo = repo.to_string();

        let github = GitHubClient::new(&config).await.ok();
        let mut last_sha = String::new();

        while running.load(Ordering::SeqCst) {
            if let Some(gh) = &github {
                if let Ok(repos) = gh.list_repos(&repo).await {
                    if let Some(r) = repos.first() {
                        let current_sha = r.default_branch.clone();
                        if !last_sha.is_empty() && current_sha != last_sha {
                            println!("New commit detected, deploying {}...", app_name);
                            let deploy_engine =
                                DeployEngine::new(&docker, gh, &state, &config);
                            let _ = deploy_engine
                                .deploy(&repo, &app_name, &app_name, None, None, None)
                                .await;
                        }
                        last_sha = current_sha;
                    }
                }
            }
            tokio::time::sleep(Duration::from_secs(config.deploy.poll_interval_secs)).await;
        }

        Ok(())
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }
}
