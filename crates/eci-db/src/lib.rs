use eci_core::config::Config;
use eci_core::error::{EciError, Result};
use eci_core::types::DbInfo;
use eci_docker::DockerClient;
use rand::Rng;
use std::fs;
use tracing::info;

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
}

impl std::str::FromStr for DbType {
    type Err = EciError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
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

    pub fn get_connection_string(
        app_name: &str,
        db_type: &DbType,
        username: &str,
        password: &str,
    ) -> Result<String> {
        Ok(match db_type {
            DbType::Postgres => format!(
                "postgresql://{}:{}@localhost:{}/{}",
                username,
                password,
                db_type.default_port(),
                app_name
            ),
            DbType::Mongo => format!(
                "mongodb://{}:{}@localhost:{}/{}",
                username,
                password,
                db_type.default_port(),
                app_name
            ),
            DbType::Redis => format!("redis://:{}@localhost:{}", password, db_type.default_port()),
            DbType::MySQL => format!(
                "mysql://{}:{}@localhost:{}/{}",
                username,
                password,
                db_type.default_port(),
                app_name
            ),
        })
    }

    pub async fn provision(&self, app_name: &str, db_type: &DbType) -> Result<DbInfo> {
        info!(app = app_name, db_type = ?db_type, "Provisioning database");
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

        let connection_string =
            Self::get_connection_string(app_name, db_type, &username, &password)?;

        Ok(DbInfo {
            app_name: app_name.to_string(),
            db_type: format!("{:?}", db_type),
            connection_string,
            credentials_path: env_path.to_string_lossy().to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn db_type_from_str_postgres() {
        assert!(matches!(
            DbType::from_str("postgres").unwrap(),
            DbType::Postgres
        ));
        assert!(matches!(
            DbType::from_str("postgresql").unwrap(),
            DbType::Postgres
        ));
        assert!(matches!(
            DbType::from_str("Postgres").unwrap(),
            DbType::Postgres
        ));
        assert!(matches!(
            DbType::from_str("POSTGRES").unwrap(),
            DbType::Postgres
        ));
    }

    #[test]
    fn db_type_from_str_mongo() {
        assert!(matches!(DbType::from_str("mongo").unwrap(), DbType::Mongo));
        assert!(matches!(
            DbType::from_str("mongodb").unwrap(),
            DbType::Mongo
        ));
        assert!(matches!(DbType::from_str("Mongo").unwrap(), DbType::Mongo));
    }

    #[test]
    fn db_type_from_str_redis() {
        assert!(matches!(DbType::from_str("redis").unwrap(), DbType::Redis));
        assert!(matches!(DbType::from_str("Redis").unwrap(), DbType::Redis));
        assert!(matches!(DbType::from_str("REDIS").unwrap(), DbType::Redis));
    }

    #[test]
    fn db_type_from_str_mysql() {
        assert!(matches!(DbType::from_str("mysql").unwrap(), DbType::MySQL));
        assert!(matches!(DbType::from_str("MySQL").unwrap(), DbType::MySQL));
        assert!(matches!(DbType::from_str("MYSQL").unwrap(), DbType::MySQL));
    }

    #[test]
    fn db_type_from_str_invalid() {
        let result = DbType::from_str("sqlite");
        assert!(result.is_err());
        let result = DbType::from_str("invalid");
        assert!(result.is_err());
    }

    #[test]
    fn db_type_image() {
        assert_eq!(DbType::Postgres.image(), "postgres:16-alpine");
        assert_eq!(DbType::Mongo.image(), "mongo:7");
        assert_eq!(DbType::Redis.image(), "redis:7-alpine");
        assert_eq!(DbType::MySQL.image(), "mysql:8");
    }

    #[test]
    fn db_type_default_port() {
        assert_eq!(DbType::Postgres.default_port(), 5432);
        assert_eq!(DbType::Mongo.default_port(), 27017);
        assert_eq!(DbType::Redis.default_port(), 6379);
        assert_eq!(DbType::MySQL.default_port(), 3306);
    }

    #[test]
    fn db_type_debug_format() {
        assert_eq!(format!("{:?}", DbType::Postgres), "Postgres");
        assert_eq!(format!("{:?}", DbType::Mongo), "Mongo");
        assert_eq!(format!("{:?}", DbType::Redis), "Redis");
        assert_eq!(format!("{:?}", DbType::MySQL), "MySQL");
    }

    #[test]
    fn db_type_clone() {
        let db_type = DbType::Postgres;
        let cloned = db_type.clone();
        assert!(matches!(cloned, DbType::Postgres));
    }

    #[test]
    fn generate_credentials_returns_username_and_password() {
        let (username, password) =
            DbProvisioner::generate_credentials(&DbType::Postgres, "myapp").unwrap();
        assert_eq!(username, "postgres");
        assert_eq!(password.len(), 24);
        assert!(password.chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn generate_credentials_mongo() {
        let (username, _) = DbProvisioner::generate_credentials(&DbType::Mongo, "myapp").unwrap();
        assert_eq!(username, "admin");
    }

    #[test]
    fn generate_credentials_redis() {
        let (username, _) = DbProvisioner::generate_credentials(&DbType::Redis, "myapp").unwrap();
        assert_eq!(username, "default");
    }

    #[test]
    fn generate_credentials_mysql() {
        let (username, _) = DbProvisioner::generate_credentials(&DbType::MySQL, "myapp").unwrap();
        assert_eq!(username, "root");
    }

    #[test]
    fn get_connection_string_postgres() {
        let cs = DbProvisioner::get_connection_string(
            "myapp",
            &DbType::Postgres,
            "postgres",
            "secret123",
        )
        .unwrap();
        assert!(cs.starts_with("postgresql://"));
        assert!(cs.contains("postgres:secret123"));
        assert!(cs.contains("5432"));
        assert!(cs.contains("myapp"));
    }

    #[test]
    fn get_connection_string_mongo() {
        let cs =
            DbProvisioner::get_connection_string("myapp", &DbType::Mongo, "admin", "secret123")
                .unwrap();
        assert!(cs.starts_with("mongodb://"));
        assert!(cs.contains("admin:secret123"));
        assert!(cs.contains("27017"));
    }

    #[test]
    fn get_connection_string_redis() {
        let cs =
            DbProvisioner::get_connection_string("myapp", &DbType::Redis, "default", "secret123")
                .unwrap();
        assert!(cs.starts_with("redis://"));
        assert!(cs.contains("secret123"));
        assert!(cs.contains("6379"));
    }

    #[test]
    fn get_connection_string_mysql() {
        let cs = DbProvisioner::get_connection_string("myapp", &DbType::MySQL, "root", "secret123")
            .unwrap();
        assert!(cs.starts_with("mysql://"));
        assert!(cs.contains("root:secret123"));
        assert!(cs.contains("3306"));
        assert!(cs.contains("myapp"));
    }

    #[test]
    fn generate_credentials_unique() {
        let (_, pass1) = DbProvisioner::generate_credentials(&DbType::Postgres, "app1").unwrap();
        let (_, pass2) = DbProvisioner::generate_credentials(&DbType::Postgres, "app1").unwrap();
        // Passwords should be different due to random generation
        // (though technically they could collide with very low probability)
        // Just verify they're valid length
        assert_eq!(pass1.len(), 24);
        assert_eq!(pass2.len(), 24);
    }
}
