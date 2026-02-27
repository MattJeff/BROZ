use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_db")]
    pub database_url: String,
    #[serde(default = "default_rabbitmq")]
    pub rabbitmq_url: String,
    #[serde(default = "default_redis")]
    pub redis_url: String,
    #[serde(default = "default_jwt_secret")]
    pub jwt_secret: String,
    #[serde(default = "default_minio_endpoint")]
    pub minio_endpoint: String,
    #[serde(default = "default_minio_access_key")]
    pub minio_access_key: String,
    #[serde(default = "default_minio_secret_key")]
    pub minio_secret_key: String,
    #[serde(default = "default_minio_bucket")]
    pub minio_bucket: String,
    #[serde(default = "default_minio_public_url")]
    pub minio_public_url: String,
}

fn default_port() -> u16 { 3002 }
fn default_db() -> String { "postgres://brozadmin:password@localhost:5432/broz_user".into() }
fn default_rabbitmq() -> String { "amqp://guest:guest@localhost:5672/%2f".into() }
fn default_redis() -> String { "redis://localhost:6379".into() }
fn default_jwt_secret() -> String { "development-secret-change-in-production".into() }
fn default_minio_endpoint() -> String { "http://localhost:9000".into() }
fn default_minio_access_key() -> String { "minioadmin".into() }
fn default_minio_secret_key() -> String { "minioadmin".into() }
fn default_minio_bucket() -> String { "broz-media".into() }
fn default_minio_public_url() -> String { "http://localhost:9000".into() }

impl AppConfig {
    pub fn load() -> anyhow::Result<Self> {
        let config = config::Config::builder()
            .add_source(config::Environment::with_prefix("BROZ_USER").separator("__"))
            .build()?;
        Ok(config.try_deserialize().unwrap_or_else(|_| Self {
            port: default_port(),
            database_url: default_db(),
            rabbitmq_url: default_rabbitmq(),
            redis_url: default_redis(),
            jwt_secret: default_jwt_secret(),
            minio_endpoint: default_minio_endpoint(),
            minio_access_key: default_minio_access_key(),
            minio_secret_key: default_minio_secret_key(),
            minio_bucket: default_minio_bucket(),
            minio_public_url: default_minio_public_url(),
        }))
    }
}
