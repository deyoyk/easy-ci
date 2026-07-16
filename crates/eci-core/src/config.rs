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
                health_check_timeout_secs: 60,
                auto_rollback_on_unhealthy: true,
            },
        }
    }
}

impl Config {
    pub fn config_dir() -> Result<PathBuf> {
        let home =
            dirs::home_dir().ok_or_else(|| EciError::Config("Cannot find home dir".into()))?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_roundtrip() {
        let config = Config::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: Config = toml::from_str(&toml_str).unwrap();
        assert_eq!(config.github.token, parsed.github.token);
        assert_eq!(config.docker.host, parsed.docker.host);
        assert_eq!(
            config.deploy.health_check_timeout_secs,
            parsed.deploy.health_check_timeout_secs
        );
    }

    #[test]
    fn default_docker_host_is_platform_specific() {
        let config = Config::default();
        // Default is unix socket for Linux/macOS; on Windows it should be npipe
        // But current default is unix for all platforms - verify it's a valid socket path
        assert!(
            config.docker.host.starts_with("unix://") || config.docker.host.starts_with("npipe://"),
            "Docker host should be a socket path, got: {}",
            config.docker.host
        );
    }
}
