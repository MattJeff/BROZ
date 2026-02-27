use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_jwt_secret")]
    pub jwt_secret: String,
    #[serde(default = "default_redis_url")]
    pub redis_url: String,

    // Downstream service URLs
    #[serde(default = "default_auth_url")]
    pub auth_url: String,
    #[serde(default = "default_user_url")]
    pub user_url: String,
    #[serde(default = "default_matching_url")]
    pub matching_url: String,
    #[serde(default = "default_messaging_url")]
    pub messaging_url: String,
    #[serde(default = "default_notification_url")]
    pub notification_url: String,
    #[serde(default = "default_moderation_url")]
    pub moderation_url: String,
    #[serde(default = "default_analytics_url")]
    pub analytics_url: String,

    // Rate limits
    #[serde(default = "default_free_rpm")]
    pub free_rpm: u64,
    #[serde(default = "default_free_rph")]
    pub free_rph: u64,
    #[serde(default = "default_premium_rpm")]
    pub premium_rpm: u64,
    #[serde(default = "default_premium_rph")]
    pub premium_rph: u64,
}

fn default_port() -> u16 { 3000 }
fn default_jwt_secret() -> String { "development-secret-change-in-production".into() }
fn default_redis_url() -> String { "redis://localhost:6379".into() }
fn default_auth_url() -> String { "http://localhost:3001".into() }
fn default_user_url() -> String { "http://localhost:3002".into() }
fn default_matching_url() -> String { "http://localhost:3003".into() }
fn default_messaging_url() -> String { "http://localhost:3004".into() }
fn default_notification_url() -> String { "http://localhost:3005".into() }
fn default_moderation_url() -> String { "http://localhost:3006".into() }
fn default_analytics_url() -> String { "http://localhost:3007".into() }
fn default_free_rpm() -> u64 { 60 }
fn default_free_rph() -> u64 { 600 }
fn default_premium_rpm() -> u64 { 300 }
fn default_premium_rph() -> u64 { 3000 }

impl AppConfig {
    pub fn load() -> anyhow::Result<Self> {
        let config = config::Config::builder()
            .add_source(config::Environment::with_prefix("BROZ_GATEWAY").separator("__"))
            .build()?;
        Ok(config.try_deserialize().unwrap_or_else(|_| Self {
            port: default_port(),
            jwt_secret: default_jwt_secret(),
            redis_url: default_redis_url(),
            auth_url: default_auth_url(),
            user_url: default_user_url(),
            matching_url: default_matching_url(),
            messaging_url: default_messaging_url(),
            notification_url: default_notification_url(),
            moderation_url: default_moderation_url(),
            analytics_url: default_analytics_url(),
            free_rpm: default_free_rpm(),
            free_rph: default_free_rph(),
            premium_rpm: default_premium_rpm(),
            premium_rph: default_premium_rph(),
        }))
    }

    /// Resolve the upstream service base URL from the incoming request path.
    pub fn resolve_upstream(&self, path: &str) -> Option<&str> {
        if path.starts_with("/api/auth/") || path == "/api/auth" {
            Some(&self.auth_url)
        } else if path.starts_with("/api/users/") || path == "/api/users" {
            Some(&self.user_url)
        } else if path.starts_with("/api/follows/") || path == "/api/follows" {
            Some(&self.user_url)
        } else if path.starts_with("/api/messages/") || path == "/api/messages" {
            Some(&self.messaging_url)
        } else if path.starts_with("/api/notifications/") || path == "/api/notifications" {
            Some(&self.notification_url)
        } else if path.starts_with("/api/interactions/") || path == "/api/interactions" {
            Some(&self.moderation_url)
        } else if path.starts_with("/api/livecam/") || path == "/api/livecam" {
            Some(&self.matching_url)
        } else if path.starts_with("/api/admin/") || path == "/api/admin" {
            Some(&self.moderation_url)
        } else if path.starts_with("/api/analytics/") || path == "/api/analytics" {
            Some(&self.analytics_url)
        } else {
            None
        }
    }
}
