use bollard::container::{
    ListContainersOptions, LogsOptions, RemoveContainerOptions, StopContainerOptions,
};
use bollard::image::BuildImageOptions;
use bollard::Docker;
use eci_core::config::DockerConfig;
use eci_core::error::{EciError, Result};
use eci_core::types::AppStatus;
use futures_util::stream::TryStreamExt;
use std::collections::HashMap;
use std::path::Path;
use tar::Builder as TarBuilder;

#[derive(Clone)]
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
    pub async fn new(config: &DockerConfig) -> Result<Self> {
        let docker = if config.host.starts_with("tcp://") {
            Docker::connect_with_local_defaults()
                .map_err(|e| EciError::Docker(format!("Docker connect to '{}' failed: {}", config.host, e)))?
        } else {
            Docker::connect_with_local_defaults()
                .map_err(|e| EciError::Docker(format!("Local Docker connect failed: {}", e)))?
        };
        Ok(Self { docker })
    }

    pub async fn build_image(&self, app_name: &str, dockerfile_path: &Path) -> Result<String> {
        let context_path = dockerfile_path
            .parent()
            .ok_or_else(|| EciError::Docker("Invalid Dockerfile path".into()))?;

        let tar_path = std::env::temp_dir().join(format!("{}.tar", app_name));
        let tar_file = std::fs::File::create(&tar_path)?;
        let mut tar = TarBuilder::new(tar_file);
        tar.append_dir_all(".", context_path)?;
        tar.finish()?;

        let build_opts = BuildImageOptions {
            dockerfile: "Dockerfile",
            t: app_name,
            rm: true,
            ..Default::default()
        };

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

    pub async fn tag_image(&self, source: &str, target: &str) -> Result<()> {
        use bollard::image::TagImageOptions;
        self.docker
            .tag_image(
                source,
                Some(TagImageOptions {
                    repo: target.to_string(),
                    ..Default::default()
                }),
            )
            .await
            .map_err(|e| {
                EciError::Docker(format!("Tag image '{}' as '{}' failed: {}", source, target, e))
            })?;
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
                                    p.public_port.map(|pp| format!("{}:{}", pp, p.private_port))
                                })
                                .collect()
                        })
                        .unwrap_or_default(),
                }
            })
            .collect())
    }
}
