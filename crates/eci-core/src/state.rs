use crate::error::Result;
use crate::types::{App, AppStatus, Deployment, Project};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use std::fs;
use tracing::{debug, info};

pub struct State {
    conn: Connection,
}

impl State {
    pub fn new() -> Result<Self> {
        let config_dir = crate::config::Config::config_dir()?;
        fs::create_dir_all(&config_dir)?;
        let db_path = config_dir.join("state.db");
        debug!("Opening state database at {}", db_path.display());
        let conn = Connection::open(db_path)?;
        let state = Self { conn };
        state.init_tables()?;
        Ok(state)
    }

    fn init_tables(&self) -> Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS projects (
                name TEXT PRIMARY KEY,
                description TEXT,
                created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS apps (
                name TEXT PRIMARY KEY,
                project_name TEXT NOT NULL,
                repo TEXT NOT NULL,
                description TEXT,
                image_tag TEXT NOT NULL,
                container_id TEXT,
                port INTEGER,
                status TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                FOREIGN KEY (project_name) REFERENCES projects(name)
            );
            CREATE TABLE IF NOT EXISTS deployments (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                app_name TEXT NOT NULL,
                version TEXT NOT NULL,
                image_tag TEXT NOT NULL,
                status TEXT NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY (app_name) REFERENCES apps(name)
            );",
        )?;
        Ok(())
    }

    fn parse_dt(s: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(s)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now())
    }

    pub fn create_project(&self, name: &str, description: Option<&str>) -> Result<Project> {
        info!(name = name, "Creating project");
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO projects (name, description, created_at) VALUES (?1, ?2, ?3)",
            params![name, description, now],
        )?;
        Ok(Project {
            name: name.to_string(),
            description: description.map(String::from),
            created_at: Utc::now(),
        })
    }

    pub fn list_projects(&self) -> Result<Vec<Project>> {
        debug!("Listing all projects");
        let mut stmt = self
            .conn
            .prepare("SELECT name, description, created_at FROM projects ORDER BY name")?;
        let projects = stmt
            .query_map([], |row| {
                let created_at_str: String = row.get(2)?;
                Ok(Project {
                    name: row.get(0)?,
                    description: row.get(1)?,
                    created_at: Self::parse_dt(&created_at_str),
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(projects)
    }

    pub fn delete_project(&self, name: &str) -> Result<bool> {
        info!(name = name, "Deleting project");
        let rows = self
            .conn
            .execute("DELETE FROM projects WHERE name = ?1", params![name])?;
        Ok(rows > 0)
    }

    pub fn create_app(
        &self,
        name: &str,
        project_name: &str,
        repo: &str,
        description: Option<&str>,
        image_tag: &str,
    ) -> Result<App> {
        info!(
            name = name,
            project = project_name,
            repo = repo,
            "Creating app"
        );
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO apps (name, project_name, repo, description, image_tag, status, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![name, project_name, repo, description, image_tag, "deploying", now, now],
        )?;
        Ok(App {
            name: name.to_string(),
            project_name: project_name.to_string(),
            repo: repo.to_string(),
            description: description.map(String::from),
            image_tag: image_tag.to_string(),
            container_id: None,
            port: None,
            status: AppStatus::Deploying,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
    }

    pub fn list_apps(&self) -> Result<Vec<App>> {
        let mut stmt = self.conn.prepare(
            "SELECT name, project_name, repo, description, image_tag, container_id, port, status, created_at, updated_at
             FROM apps ORDER BY name",
        )?;
        let apps = stmt
            .query_map([], |row| {
                let status_str: String = row.get(7)?;
                let status = match status_str.as_str() {
                    "running" => AppStatus::Running,
                    "stopped" => AppStatus::Stopped,
                    "unhealthy" => AppStatus::Unhealthy,
                    _ => AppStatus::Deploying,
                };
                let created_at_str: String = row.get(8)?;
                let updated_at_str: String = row.get(9)?;
                Ok(App {
                    name: row.get(0)?,
                    project_name: row.get(1)?,
                    repo: row.get(2)?,
                    description: row.get(3)?,
                    image_tag: row.get(4)?,
                    container_id: row.get(5)?,
                    port: row.get(6)?,
                    status,
                    created_at: Self::parse_dt(&created_at_str),
                    updated_at: Self::parse_dt(&updated_at_str),
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(apps)
    }

    pub fn update_app_status(&self, name: &str, status: &AppStatus) -> Result<()> {
        let status_str = match status {
            AppStatus::Running => "running",
            AppStatus::Stopped => "stopped",
            AppStatus::Unhealthy => "unhealthy",
            AppStatus::Deploying => "deploying",
        };
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE apps SET status = ?1, updated_at = ?2 WHERE name = ?3",
            params![status_str, now, name],
        )?;
        Ok(())
    }

    pub fn get_app(&self, name: &str) -> Result<Option<App>> {
        let mut stmt = self.conn.prepare(
            "SELECT name, project_name, repo, description, image_tag, container_id, port, status, created_at, updated_at
             FROM apps WHERE name = ?1",
        )?;
        let mut rows = stmt.query_map(params![name], |row| {
            let status_str: String = row.get(7)?;
            let status = match status_str.as_str() {
                "running" => AppStatus::Running,
                "stopped" => AppStatus::Stopped,
                "unhealthy" => AppStatus::Unhealthy,
                _ => AppStatus::Deploying,
            };
            let created_at_str: String = row.get(8)?;
            let updated_at_str: String = row.get(9)?;
            Ok(App {
                name: row.get(0)?,
                project_name: row.get(1)?,
                repo: row.get(2)?,
                description: row.get(3)?,
                image_tag: row.get(4)?,
                container_id: row.get(5)?,
                port: row.get(6)?,
                status,
                created_at: Self::parse_dt(&created_at_str),
                updated_at: Self::parse_dt(&updated_at_str),
            })
        })?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    pub fn delete_app(&self, name: &str) -> Result<bool> {
        info!(name = name, "Deleting app");
        let rows = self
            .conn
            .execute("DELETE FROM apps WHERE name = ?1", params![name])?;
        Ok(rows > 0)
    }

    pub fn create_deployment(
        &self,
        app_name: &str,
        version: &str,
        image_tag: &str,
    ) -> Result<Deployment> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO deployments (app_name, version, image_tag, status, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![app_name, version, image_tag, "deployed", now],
        )?;
        Ok(Deployment {
            id: self.conn.last_insert_rowid(),
            app_name: app_name.to_string(),
            version: version.to_string(),
            image_tag: image_tag.to_string(),
            status: "deployed".to_string(),
            created_at: Utc::now(),
        })
    }

    pub fn list_deployments(&self, app_name: &str) -> Result<Vec<Deployment>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, app_name, version, image_tag, status, created_at FROM deployments WHERE app_name = ?1 ORDER BY created_at DESC LIMIT 10",
        )?;
        let deployments = stmt
            .query_map(params![app_name], |row| {
                let created_at_str: String = row.get(5)?;
                Ok(Deployment {
                    id: row.get(0)?,
                    app_name: row.get(1)?,
                    version: row.get(2)?,
                    image_tag: row.get(3)?,
                    status: row.get(4)?,
                    created_at: Self::parse_dt(&created_at_str),
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(deployments)
    }

    pub fn get_latest_deployment(&self, app_name: &str) -> Result<Option<Deployment>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, app_name, version, image_tag, status, created_at FROM deployments WHERE app_name = ?1 ORDER BY created_at DESC LIMIT 1",
        )?;
        let mut rows = stmt.query_map(params![app_name], |row| {
            let created_at_str: String = row.get(5)?;
            Ok(Deployment {
                id: row.get(0)?,
                app_name: row.get(1)?,
                version: row.get(2)?,
                image_tag: row.get(3)?,
                status: row.get(4)?,
                created_at: Self::parse_dt(&created_at_str),
            })
        })?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unique_name(prefix: &str) -> String {
        format!("{}-{}", prefix, chrono::Utc::now().timestamp_millis())
    }

    #[test]
    fn project_crud() {
        let state = State::new().unwrap();
        let name = unique_name("test-proj");
        state.create_project(&name, Some("test desc")).unwrap();
        let projects = state.list_projects().unwrap();
        assert!(projects.iter().any(|p| p.name == name));
        state.delete_project(&name).unwrap();
        let projects = state.list_projects().unwrap();
        assert!(!projects.iter().any(|p| p.name == name));
    }

    #[test]
    fn app_crud() {
        let state = State::new().unwrap();
        let proj = unique_name("test-proj");
        let app = unique_name("test-app");
        state.create_project(&proj, None).unwrap();
        state
            .create_app(&app, &proj, "owner/repo", None, "test:latest")
            .unwrap();
        let result = state.get_app(&app).unwrap();
        assert!(result.is_some());
        state.update_app_status(&app, &AppStatus::Running).unwrap();
        let result = state.get_app(&app).unwrap().unwrap();
        assert_eq!(result.status, AppStatus::Running);
        state.delete_app(&app).unwrap();
        assert!(state.get_app(&app).unwrap().is_none());
    }

    #[test]
    fn deployment_crud() {
        let state = State::new().unwrap();
        let proj = unique_name("test-proj");
        let app = unique_name("test-app");
        state.create_project(&proj, None).unwrap();
        state
            .create_app(&app, &proj, "owner/repo", None, "test:latest")
            .unwrap();
        state.create_deployment(&app, "v1", "test:v1").unwrap();
        let deps = state.list_deployments(&app).unwrap();
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].version, "v1");
        let latest = state.get_latest_deployment(&app).unwrap();
        assert!(latest.is_some());
        assert_eq!(latest.unwrap().version, "v1");
    }

    #[test]
    fn project_create_with_description() {
        let state = State::new().unwrap();
        let name = unique_name("test-proj-desc");
        let project = state.create_project(&name, Some("My description")).unwrap();
        assert_eq!(project.name, name);
        assert_eq!(project.description, Some("My description".to_string()));
    }

    #[test]
    fn project_create_without_description() {
        let state = State::new().unwrap();
        let name = unique_name("test-proj-nodesc");
        let project = state.create_project(&name, None).unwrap();
        assert_eq!(project.name, name);
        assert!(project.description.is_none());
    }

    #[test]
    fn list_projects_empty() {
        let state = State::new().unwrap();
        let projects = state.list_projects().unwrap();
        // Should not fail, just return empty or existing projects
        let _ = projects.len();
    }

    #[test]
    fn list_projects_multiple() {
        let state = State::new().unwrap();
        let name1 = unique_name("proj-a");
        let name2 = unique_name("proj-b");
        state.create_project(&name1, None).unwrap();
        state.create_project(&name2, None).unwrap();
        let projects = state.list_projects().unwrap();
        let names: Vec<&str> = projects.iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains(&name1.as_str()));
        assert!(names.contains(&name2.as_str()));
    }

    #[test]
    fn delete_nonexistent_project() {
        let state = State::new().unwrap();
        let result = state.delete_project("nonexistent-project-12345").unwrap();
        assert!(!result);
    }

    #[test]
    fn delete_nonexistent_app() {
        let state = State::new().unwrap();
        let result = state.delete_app("nonexistent-app-12345").unwrap();
        assert!(!result);
    }

    #[test]
    fn get_nonexistent_app() {
        let state = State::new().unwrap();
        let result = state.get_app("nonexistent-app-12345").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn app_create_with_all_fields() {
        let state = State::new().unwrap();
        let proj = unique_name("test-proj-full");
        let app = unique_name("test-app-full");
        state.create_project(&proj, None).unwrap();
        let created = state
            .create_app(&app, &proj, "owner/repo", Some("My app"), "my-app:v1")
            .unwrap();
        assert_eq!(created.name, app);
        assert_eq!(created.project_name, proj);
        assert_eq!(created.repo, "owner/repo");
        assert_eq!(created.description, Some("My app".to_string()));
        assert_eq!(created.image_tag, "my-app:v1");
        assert_eq!(created.status, AppStatus::Deploying);
        assert!(created.container_id.is_none());
        assert!(created.port.is_none());
    }

    #[test]
    fn update_app_status_all_variants() {
        let state = State::new().unwrap();
        let proj = unique_name("test-proj-status");
        let app = unique_name("test-app-status");
        state.create_project(&proj, None).unwrap();
        state
            .create_app(&app, &proj, "owner/repo", None, "test:latest")
            .unwrap();

        state.update_app_status(&app, &AppStatus::Running).unwrap();
        assert_eq!(
            state.get_app(&app).unwrap().unwrap().status,
            AppStatus::Running
        );

        state.update_app_status(&app, &AppStatus::Stopped).unwrap();
        assert_eq!(
            state.get_app(&app).unwrap().unwrap().status,
            AppStatus::Stopped
        );

        state
            .update_app_status(&app, &AppStatus::Unhealthy)
            .unwrap();
        assert_eq!(
            state.get_app(&app).unwrap().unwrap().status,
            AppStatus::Unhealthy
        );

        state
            .update_app_status(&app, &AppStatus::Deploying)
            .unwrap();
        assert_eq!(
            state.get_app(&app).unwrap().unwrap().status,
            AppStatus::Deploying
        );
    }

    #[test]
    fn multiple_deployments_ordering() {
        let state = State::new().unwrap();
        let proj = unique_name("test-proj-multi");
        let app = unique_name("test-app-multi");
        state.create_project(&proj, None).unwrap();
        state
            .create_app(&app, &proj, "owner/repo", None, "test:latest")
            .unwrap();

        state.create_deployment(&app, "v1", "test:v1").unwrap();
        state.create_deployment(&app, "v2", "test:v2").unwrap();
        state.create_deployment(&app, "v3", "test:v3").unwrap();

        let deps = state.list_deployments(&app).unwrap();
        assert_eq!(deps.len(), 3);
        // Should be ordered by created_at DESC
        assert_eq!(deps[0].version, "v3");
        assert_eq!(deps[1].version, "v2");
        assert_eq!(deps[2].version, "v1");

        let latest = state.get_latest_deployment(&app).unwrap().unwrap();
        assert_eq!(latest.version, "v3");
    }

    #[test]
    fn list_deployments_empty() {
        let state = State::new().unwrap();
        let deps = state.list_deployments("nonexistent-app").unwrap();
        assert!(deps.is_empty());
    }

    #[test]
    fn get_latest_deployment_none() {
        let state = State::new().unwrap();
        let result = state.get_latest_deployment("nonexistent-app").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn deployment_fields() {
        let state = State::new().unwrap();
        let proj = unique_name("test-proj-depfields");
        let app = unique_name("test-app-depfields");
        state.create_project(&proj, None).unwrap();
        state
            .create_app(&app, &proj, "owner/repo", None, "test:latest")
            .unwrap();

        let dep = state.create_deployment(&app, "v42", "img:v42").unwrap();
        assert_eq!(dep.app_name, app);
        assert_eq!(dep.version, "v42");
        assert_eq!(dep.image_tag, "img:v42");
        assert_eq!(dep.status, "deployed");
        assert!(dep.id > 0);
    }
}
