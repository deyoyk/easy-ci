use crate::error::Result;
use crate::types::{App, AppStatus, Project};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use std::fs;

pub struct State {
    conn: Connection,
}

impl State {
    pub fn new() -> Result<Self> {
        let config_dir = crate::config::Config::config_dir()?;
        fs::create_dir_all(&config_dir)?;
        let db_path = config_dir.join("state.db");
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
        let rows = self
            .conn
            .execute("DELETE FROM apps WHERE name = ?1", params![name])?;
        Ok(rows > 0)
    }
}
