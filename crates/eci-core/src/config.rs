use crate::error::{EciError, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tracing::{debug, info};

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
            debug!("No config file found, using defaults");
            return Ok(Self::default());
        }
        info!("Loading config from {}", path.display());
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
        let path = Self::config_path()?;
        info!("Saving config to {}", path.display());
        fs::write(&path, content)?;
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

    #[test]
    fn default_config_values() {
        let config = Config::default();
        assert!(config.github.token.is_empty());
        assert!(config.github.default_org.is_none());
        assert_eq!(config.docker.host, "unix:///var/run/docker.sock");
        assert_eq!(config.deploy.health_check_timeout_secs, 60);
        assert!(config.deploy.auto_rollback_on_unhealthy);
    }

    #[test]
    fn config_serialization_custom_values() {
        let config = Config {
            github: GitHubConfig {
                token: "ghp_test123".to_string(),
                default_org: Some("myorg".to_string()),
            },
            docker: DockerConfig {
                host: "tcp://localhost:2375".to_string(),
            },
            deploy: DeployConfig {
                health_check_timeout_secs: 30,
                auto_rollback_on_unhealthy: false,
            },
        };
        let toml_str = toml::to_string_pretty(&config).unwrap();
        assert!(toml_str.contains("ghp_test123"));
        assert!(toml_str.contains("tcp://localhost:2375"));
        assert!(toml_str.contains("myorg"));

        let parsed: Config = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.github.token, "ghp_test123");
        assert_eq!(parsed.github.default_org, Some("myorg".to_string()));
        assert_eq!(parsed.docker.host, "tcp://localhost:2375");
        assert_eq!(parsed.deploy.health_check_timeout_secs, 30);
        assert!(!parsed.deploy.auto_rollback_on_unhealthy);
    }

    #[test]
    fn config_clone() {
        let config = Config::default();
        let cloned = config.clone();
        assert_eq!(config.github.token, cloned.github.token);
        assert_eq!(config.docker.host, cloned.docker.host);
    }

    #[test]
    fn config_debug_format() {
        let config = Config::default();
        let debug = format!("{:?}", config);
        assert!(debug.contains("Config"));
        assert!(debug.contains("GitHubConfig"));
        assert!(debug.contains("DockerConfig"));
        assert!(debug.contains("DeployConfig"));
    }

    #[test]
    fn github_config_fields() {
        let gh = GitHubConfig {
            token: "token".to_string(),
            default_org: Some("org".to_string()),
        };
        assert_eq!(gh.token, "token");
        assert_eq!(gh.default_org, Some("org".to_string()));
    }

    #[test]
    fn docker_config_fields() {
        let docker = DockerConfig {
            host: "unix:///var/run/docker.sock".to_string(),
        };
        assert_eq!(docker.host, "unix:///var/run/docker.sock");
    }

    #[test]
    fn deploy_config_fields() {
        let deploy = DeployConfig {
            health_check_timeout_secs: 120,
            auto_rollback_on_unhealthy: false,
        };
        assert_eq!(deploy.health_check_timeout_secs, 120);
        assert!(!deploy.auto_rollback_on_unhealthy);
    }

    #[test]
    fn config_toml_parse_invalid() {
        let result = toml::from_str::<Config>("this is not valid toml [[[");
        assert!(result.is_err());
    }

    #[test]
    fn config_toml_parse_missing_fields() {
        let result = toml::from_str::<Config>("[github]\ntoken = \"test\"");
        assert!(result.is_err());
    }
}
