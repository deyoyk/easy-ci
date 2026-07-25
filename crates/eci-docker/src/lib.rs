use bollard::container::LogsOptions;
use bollard::Docker;
use eci_core::config::DockerConfig;
use eci_core::error::{EciError, Result};
use eci_core::types::AppStatus;
use futures_util::stream::TryStreamExt;
use std::path::Path;
use tracing::{debug, info};

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
        info!(host = %config.host, "Connecting to Docker");
        let docker = if config.host.starts_with("tcp://") {
            let addr = config.host.trim_start_matches("tcp://");
            debug!("Connecting to remote Docker host via TCP: {}", addr);
            Docker::connect_with_http(addr, 120, bollard::API_DEFAULT_VERSION).map_err(|e| {
                EciError::Docker(format!("Docker connect to '{}' failed: {}", config.host, e))
            })?
        } else {
            debug!("Connecting to local Docker socket");
            Docker::connect_with_local_defaults()
                .map_err(|e| EciError::Docker(format!("Local Docker connect failed: {}", e)))?
        };
        Ok(Self { docker })
    }

    pub async fn build_image(&self, app_name: &str, dockerfile_path: &Path) -> Result<String> {
        use std::process::Command;

        if !dockerfile_path.exists() {
            return Err(EciError::Docker(format!(
                "Dockerfile not found at '{}'",
                dockerfile_path.display()
            )));
        }

        let context_path = dockerfile_path
            .parent()
            .ok_or_else(|| EciError::Docker("Invalid Dockerfile path".into()))?;

        info!(app = app_name, context = %context_path.display(), "Building Docker image");

        let output = Command::new("docker")
            .arg("build")
            .arg("-t")
            .arg(app_name)
            .arg("-f")
            .arg(dockerfile_path)
            .arg(context_path)
            .output()
            .map_err(|e| EciError::Docker(format!("Failed to run docker: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(EciError::Docker(format!("Build failed: {}", stderr.trim())));
        }

        info!(image = app_name, "Docker image built successfully");
        Ok(app_name.to_string())
    }

    pub async fn run_container(
        &self,
        app_name: &str,
        image: &str,
        port: Option<u16>,
    ) -> Result<String> {
        use std::process::Command;

        // Remove existing container with same name if any
        let _ = Command::new("docker")
            .arg("rm")
            .arg("-f")
            .arg(app_name)
            .output();

        let mut args = vec![
            "run", "-d", "--name", app_name,
        ];

        let port_arg;
        if let Some(p) = port {
            port_arg = format!("{}:80", p);
            args.push("-p");
            args.push(&port_arg);
        }

        args.push(image);

        let output = Command::new("docker")
            .args(&args)
            .output()
            .map_err(|e| EciError::Docker(format!("Failed to run docker: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(EciError::Docker(format!("Failed to start container: {}", stderr.trim())));
        }

        let container_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
        info!(container_id = %container_id, name = app_name, "Container started");
        Ok(container_id)
    }

    pub async fn stop_container(&self, container_id: &str) -> Result<()> {
        use std::process::Command;

        info!(container_id = container_id, "Stopping container");
        Command::new("docker")
            .arg("stop")
            .arg(container_id)
            .output()
            .map_err(|e| EciError::Docker(format!("Stop error: {}", e)))?;
        info!(container_id = container_id, "Container stopped");
        Ok(())
    }

    pub async fn remove_container(&self, container_id: &str) -> Result<()> {
        use std::process::Command;

        info!(container_id = container_id, "Removing container");
        Command::new("docker")
            .arg("rm")
            .arg(container_id)
            .output()
            .map_err(|e| EciError::Docker(format!("Remove error: {}", e)))?;
        info!(container_id = container_id, "Container removed");
        Ok(())
    }

    pub async fn tag_image(&self, source: &str, target: &str) -> Result<()> {
        use std::process::Command;

        debug!(source = source, target = target, "Tagging image");
        Command::new("docker")
            .arg("tag")
            .arg(source)
            .arg(target)
            .output()
            .map_err(|e| EciError::Docker(format!("Tag error: {}", e)))?;
        Ok(())
    }

    pub async fn logs(&self, container_id: &str) -> Result<Vec<String>> {
        debug!(container_id = container_id, "Fetching container logs");
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
        use std::process::Command;

        debug!("Listing all containers");
        let output = Command::new("docker")
            .args(["ps", "-a", "--format", "{{.ID}}\t{{.Names}}\t{{.Image}}\t{{.Status}}\t{{.Ports}}"])
            .output()
            .map_err(|e| EciError::Docker(format!("Failed to run docker: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut containers = Vec::new();

        for line in stdout.lines() {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 4 {
                let status = if parts[3].contains("Up") {
                    AppStatus::Running
                } else {
                    AppStatus::Stopped
                };
                let ports = if parts.len() >= 5 {
                    parts[4].split(',').map(|s| s.trim().to_string()).collect()
                } else {
                    vec![]
                };
                containers.push(ContainerInfo {
                    id: parts[0].to_string(),
                    name: parts[1].to_string(),
                    image: parts[2].to_string(),
                    status,
                    ports,
                });
            }
        }

        Ok(containers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn container_info_debug() {
        let info = ContainerInfo {
            id: "abc123".to_string(),
            name: "test-container".to_string(),
            image: "nginx:latest".to_string(),
            status: AppStatus::Running,
            ports: vec!["8080:80".to_string()],
        };
        let debug = format!("{:?}", info);
        assert!(debug.contains("abc123"));
        assert!(debug.contains("test-container"));
        assert!(debug.contains("nginx:latest"));
    }

    #[test]
    fn container_info_fields() {
        let info = ContainerInfo {
            id: "id1".to_string(),
            name: "name1".to_string(),
            image: "img1".to_string(),
            status: AppStatus::Stopped,
            ports: vec![],
        };
        assert_eq!(info.id, "id1");
        assert_eq!(info.name, "name1");
        assert_eq!(info.image, "img1");
        assert_eq!(info.status, AppStatus::Stopped);
        assert!(info.ports.is_empty());
    }

    #[test]
    fn container_info_with_ports() {
        let info = ContainerInfo {
            id: "id1".to_string(),
            name: "name1".to_string(),
            image: "img1".to_string(),
            status: AppStatus::Running,
            ports: vec!["3000:3000".to_string(), "8080:80".to_string()],
        };
        assert_eq!(info.ports.len(), 2);
        assert!(info.ports.contains(&"3000:3000".to_string()));
        assert!(info.ports.contains(&"8080:80".to_string()));
    }

    #[test]
    fn docker_client_clone() {
        // DockerClient derives Clone, verify the type implements it
        fn assert_clone<T: Clone>() {}
        assert_clone::<DockerClient>();
    }
}
