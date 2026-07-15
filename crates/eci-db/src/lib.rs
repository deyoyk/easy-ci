use eci_core::config::Config;
use eci_core::error::{EciError, Result};
use eci_core::types::DbInfo;
use eci_docker::DockerClient;
use rand::Rng;
use std::fs;

pub struct DbProvisioner<'a> {
    docker: &'a DockerClient,
    #[allow(dead_code)]
    config: &'a Config,
}

#[derive(Debug, Clone)]
pub enum DbType {
    Postgres,
    Mongo,
    Redis,
    MySQL,
}

impl DbType {
    pub fn image(&self) -> &str {
        match self {
            DbType::Postgres => "postgres:16-alpine",
            DbType::Mongo => "mongo:7",
            DbType::Redis => "redis:7-alpine",
            DbType::MySQL => "mysql:8",
        }
    }

    pub fn default_port(&self) -> u16 {
        match self {
            DbType::Postgres => 5432,
            DbType::Mongo => 27017,
            DbType::Redis => 6379,
            DbType::MySQL => 3306,
        }
    }

    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "postgres" | "postgresql" => Ok(DbType::Postgres),
            "mongo" | "mongodb" => Ok(DbType::Mongo),
            "redis" => Ok(DbType::Redis),
            "mysql" => Ok(DbType::MySQL),
            _ => Err(EciError::Database(format!("Unsupported DB type: {}", s))),
        }
    }
}

impl<'a> DbProvisioner<'a> {
    pub fn new(docker: &'a DockerClient, config: &'a Config) -> Self {
        Self { docker, config }
    }

    pub fn generate_credentials(db_type: &DbType, _app_name: &str) -> Result<(String, String)> {
        let mut rng = rand::thread_rng();
        let password: String = (0..24)
            .map(|_| {
                let idx = rng.gen_range(0..36);
                if idx < 10 {
                    (b'0' + idx) as char
                } else {
                    (b'a' + idx - 10) as char
                }
            })
            .collect();

        let username = match db_type {
            DbType::Postgres => "postgres".to_string(),
            DbType::Mongo => "admin".to_string(),
            DbType::Redis => "default".to_string(),
            DbType::MySQL => "root".to_string(),
        };

        Ok((username, password))
    }

    pub async fn provision(
        &self,
        app_name: &str,
        db_type: &DbType,
    ) -> Result<DbInfo> {
        let (username, password) = Self::generate_credentials(db_type, app_name)?;

        let secrets_dir = Config::config_dir()?.join("secrets");
        fs::create_dir_all(&secrets_dir)?;
        let env_path = secrets_dir.join(format!("{}.env", app_name));

        let env_content = match db_type {
            DbType::Postgres => format!(
                "POSTGRES_USER={}\nPOSTGRES_PASSWORD={}\nPOSTGRES_DB={}",
                username, password, app_name
            ),
            DbType::Mongo => format!(
                "MONGO_INITDB_ROOT_USERNAME={}\nMONGO_INITDB_ROOT_PASSWORD={}",
                username, password
            ),
            DbType::Redis => format!("REDIS_PASSWORD={}", password),
            DbType::MySQL => format!(
                "MYSQL_ROOT_PASSWORD={}\nMYSQL_DATABASE={}",
                password, app_name
            ),
        };

        fs::write(&env_path, env_content)?;

        let container_name = format!("eci-{}", app_name);
        let image = db_type.image();

        self.docker
            .run_container(&container_name, image, Some(db_type.default_port()))
            .await?;

        let connection_string = match db_type {
            DbType::Postgres => format!(
                "postgresql://{}:{}@localhost:{}/{}",
                username, password, db_type.default_port(), app_name
            ),
            DbType::Mongo => format!(
                "mongodb://{}:{}@localhost:{}/{}",
                username, password, db_type.default_port(), app_name
            ),
            DbType::Redis => format!(
                "redis://:{}@localhost:{}",
                password, db_type.default_port()
            ),
            DbType::MySQL => format!(
                "mysql://{}:{}@localhost:{}/{}",
                username, password, db_type.default_port(), app_name
            ),
        };

        Ok(DbInfo {
            app_name: app_name.to_string(),
            db_type: format!("{:?}", db_type),
            connection_string,
            credentials_path: env_path.to_string_lossy().to_string(),
        })
    }
}
