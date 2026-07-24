use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

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

impl fmt::Display for AppStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Running => write!(f, "running"),
            Self::Stopped => write!(f, "stopped"),
            Self::Unhealthy => write!(f, "unhealthy"),
            Self::Deploying => write!(f, "deploying"),
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_serialization_roundtrip() {
        let project = Project {
            name: "test-project".to_string(),
            description: Some("A test project".to_string()),
            created_at: Utc::now(),
        };
        let json = serde_json::to_string(&project).unwrap();
        let deserialized: Project = serde_json::from_str(&json).unwrap();
        assert_eq!(project.name, deserialized.name);
        assert_eq!(project.description, deserialized.description);
    }

    #[test]
    fn project_without_description() {
        let project = Project {
            name: "no-desc".to_string(),
            description: None,
            created_at: Utc::now(),
        };
        let json = serde_json::to_string(&project).unwrap();
        assert!(json.contains("null"));
        let deserialized: Project = serde_json::from_str(&json).unwrap();
        assert!(deserialized.description.is_none());
    }

    #[test]
    fn app_status_display() {
        assert_eq!(AppStatus::Running.to_string(), "running");
        assert_eq!(AppStatus::Stopped.to_string(), "stopped");
        assert_eq!(AppStatus::Unhealthy.to_string(), "unhealthy");
        assert_eq!(AppStatus::Deploying.to_string(), "deploying");
    }

    #[test]
    fn app_status_equality() {
        assert_eq!(AppStatus::Running, AppStatus::Running);
        assert_ne!(AppStatus::Running, AppStatus::Stopped);
        assert_ne!(AppStatus::Unhealthy, AppStatus::Deploying);
    }

    #[test]
    fn app_status_clone() {
        let status = AppStatus::Running;
        let cloned = status.clone();
        assert_eq!(status, cloned);
    }

    #[test]
    fn app_serialization_roundtrip() {
        let app = App {
            name: "my-app".to_string(),
            project_name: "my-project".to_string(),
            repo: "owner/repo".to_string(),
            description: Some("Test app".to_string()),
            image_tag: "my-app:latest".to_string(),
            container_id: Some("abc123".to_string()),
            port: Some(8080),
            status: AppStatus::Running,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let json = serde_json::to_string(&app).unwrap();
        let deserialized: App = serde_json::from_str(&json).unwrap();
        assert_eq!(app.name, deserialized.name);
        assert_eq!(app.status, deserialized.status);
        assert_eq!(app.port, deserialized.port);
    }

    #[test]
    fn app_with_optional_fields_none() {
        let app = App {
            name: "minimal".to_string(),
            project_name: "proj".to_string(),
            repo: "o/r".to_string(),
            description: None,
            image_tag: "img:latest".to_string(),
            container_id: None,
            port: None,
            status: AppStatus::Deploying,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let json = serde_json::to_string(&app).unwrap();
        let deserialized: App = serde_json::from_str(&json).unwrap();
        assert!(deserialized.description.is_none());
        assert!(deserialized.container_id.is_none());
        assert!(deserialized.port.is_none());
    }

    #[test]
    fn deployment_serialization_roundtrip() {
        let deployment = Deployment {
            id: 1,
            app_name: "test-app".to_string(),
            version: "v1".to_string(),
            image_tag: "test-app:v1".to_string(),
            status: "deployed".to_string(),
            created_at: Utc::now(),
        };
        let json = serde_json::to_string(&deployment).unwrap();
        let deserialized: Deployment = serde_json::from_str(&json).unwrap();
        assert_eq!(deployment.id, deserialized.id);
        assert_eq!(deployment.app_name, deserialized.app_name);
        assert_eq!(deployment.version, deserialized.version);
    }

    #[test]
    fn db_info_serialization_roundtrip() {
        let db_info = DbInfo {
            app_name: "my-app".to_string(),
            db_type: "Postgres".to_string(),
            connection_string: "postgresql://user:pass@localhost:5432/my-app".to_string(),
            credentials_path: "/home/user/.eci/secrets/my-app.env".to_string(),
        };
        let json = serde_json::to_string(&db_info).unwrap();
        let deserialized: DbInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(db_info.app_name, deserialized.app_name);
        assert_eq!(db_info.db_type, deserialized.db_type);
        assert_eq!(db_info.connection_string, deserialized.connection_string);
    }

    #[test]
    fn app_status_debug_format() {
        assert_eq!(format!("{:?}", AppStatus::Running), "Running");
        assert_eq!(format!("{:?}", AppStatus::Stopped), "Stopped");
        assert_eq!(format!("{:?}", AppStatus::Unhealthy), "Unhealthy");
        assert_eq!(format!("{:?}", AppStatus::Deploying), "Deploying");
    }

    #[test]
    fn app_status_clone_does_not_affect_original() {
        let mut status = AppStatus::Running;
        let cloned = status.clone();
        status = AppStatus::Stopped;
        assert_eq!(cloned, AppStatus::Running);
        assert_eq!(status, AppStatus::Stopped);
    }
}
