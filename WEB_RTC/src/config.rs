use serde::{Deserialize, Serialize};
use tracing::{info, warn};

// ---------------------------------------------------------------------------
// Production configuration — loaded from environment variables
// ---------------------------------------------------------------------------

/// Complete server configuration loaded at startup.
///
/// Every field can be set via an environment variable prefixed with
/// `LIVERELAY_`.  Defaults are suitable for local development; production
/// deployments MUST override at least `jwt_secret` and the TLS / TURN
/// settings.
#[derive(Debug, Clone)]
pub struct Config {
    // ── Network ─────────────────────────────────────────────────────────
    /// Address to bind the HTTP(S) listener to.
    pub bind_addr: String,
    /// Public hostname (used for TURN realm and TLS SNI).
    pub public_host: String,

    // ── TLS ─────────────────────────────────────────────────────────────
    /// Enable native TLS termination inside the binary.
    pub tls_enabled: bool,
    /// Path to PEM-encoded certificate chain.
    pub tls_cert_path: Option<String>,
    /// Path to PEM-encoded private key.
    pub tls_key_path: Option<String>,

    // ── TURN / STUN ─────────────────────────────────────────────────────
    /// Run an embedded TURN server inside the binary.
    pub turn_embedded: bool,
    /// UDP port for the embedded TURN relay.
    pub turn_port: u16,
    /// TURN username (shared-secret / long-term credentials).
    pub turn_username: String,
    /// TURN password.
    pub turn_password: String,
    /// TURN realm (usually the domain).
    pub turn_realm: String,

    /// STUN server URLs sent to both server-side and client-side ICE agents.
    pub stun_urls: Vec<String>,
    /// External TURN server URLs (used when `turn_embedded` is false).
    pub turn_urls: Vec<String>,

    // ── Auth ─────────────────────────────────────────────────────────────
    pub jwt_secret: String,

    // ── Limits ───────────────────────────────────────────────────────────
    /// Maximum number of rooms that can exist simultaneously.
    pub max_rooms: usize,
    /// Maximum number of subscribers per room.
    pub max_subscribers_per_room: u64,

    // ── WebRTC UDP port range (for Docker) ─────────────────────────────
    /// Minimum UDP port for WebRTC ICE candidates (0 = OS picks).
    pub udp_port_min: u16,
    /// Maximum UDP port for WebRTC ICE candidates (0 = OS picks).
    pub udp_port_max: u16,

    // ── CORS ─────────────────────────────────────────────────────────────
    pub allowed_origins: String,

    // ── Logging ──────────────────────────────────────────────────────────
    pub log_level: String,
}

impl Config {
    /// Load configuration from environment variables.
    ///
    /// Automatically loads a `.env` file if present (via `dotenvy`).
    pub fn from_env() -> Self {
        // Best-effort .env loading — ignore errors.
        let _ = dotenvy::dotenv();

        let jwt_secret = match std::env::var("LIVERELAY_JWT_SECRET") {
            Ok(s) if !s.is_empty() => {
                info!("JWT secret loaded from LIVERELAY_JWT_SECRET");
                s
            }
            _ => {
                let secret = uuid::Uuid::new_v4().to_string();
                warn!(
                    "LIVERELAY_JWT_SECRET not set — using random value (not suitable for production)"
                );
                secret
            }
        };

        let bind_addr = env_or("LIVERELAY_BIND_ADDR", "0.0.0.0:8080");
        let public_host = env_or("LIVERELAY_PUBLIC_HOST", "localhost");

        // TLS
        let tls_enabled = env_bool("LIVERELAY_TLS_ENABLED", false);
        let tls_cert_path = std::env::var("LIVERELAY_TLS_CERT_PATH").ok();
        let tls_key_path = std::env::var("LIVERELAY_TLS_KEY_PATH").ok();

        // TURN
        let turn_embedded = env_bool("LIVERELAY_TURN_EMBEDDED", false);
        let turn_port = env_or("LIVERELAY_TURN_PORT", "3478")
            .parse::<u16>()
            .unwrap_or(3478);
        let turn_username = env_or("LIVERELAY_TURN_USERNAME", "liverelay");
        let turn_password = env_or("LIVERELAY_TURN_PASSWORD", "liverelay-secret");
        let turn_realm = env_or("LIVERELAY_TURN_REALM", &public_host);

        let stun_urls = env_csv(
            "LIVERELAY_STUN_URLS",
            &["stun:stun.l.google.com:19302"],
        );
        let turn_urls = env_csv("LIVERELAY_TURN_URLS", &[]);

        // Limits
        let max_rooms = env_or("LIVERELAY_MAX_ROOMS", "100")
            .parse::<usize>()
            .unwrap_or(100);
        let max_subscribers_per_room =
            env_or("LIVERELAY_MAX_SUBSCRIBERS_PER_ROOM", "1000")
                .parse::<u64>()
                .unwrap_or(1000);

        let allowed_origins = env_or("LIVERELAY_ALLOWED_ORIGINS", "*");
        let log_level = env_or("LIVERELAY_LOG_LEVEL", "info");

        let udp_port_min = env_or("LIVERELAY_UDP_PORT_MIN", "0")
            .parse::<u16>()
            .unwrap_or(0);
        let udp_port_max = env_or("LIVERELAY_UDP_PORT_MAX", "0")
            .parse::<u16>()
            .unwrap_or(0);

        let config = Config {
            bind_addr,
            public_host,
            tls_enabled,
            tls_cert_path,
            tls_key_path,
            turn_embedded,
            turn_port,
            turn_username,
            turn_password,
            turn_realm,
            stun_urls,
            turn_urls,
            jwt_secret,
            max_rooms,
            max_subscribers_per_room,
            udp_port_min,
            udp_port_max,
            allowed_origins,
            log_level,
        };

        config.log_summary();
        config
    }

    /// Build the list of ICE servers that the server-side WebRTC agent
    /// (webrtc-rs `RTCPeerConnection`) should use.
    ///
    /// The SFU server does NOT need TURN — it has a public/routable IP.
    /// Including TURN here causes "invalid turn server credentials" errors
    /// because webrtc-rs long-term credential auth doesn't match the
    /// embedded TURN server's MD5-based auth.  Only STUN is needed for
    /// server-side ICE gathering.
    pub fn ice_servers_for_server(&self) -> Vec<IceServerConfig> {
        let mut servers: Vec<IceServerConfig> = Vec::new();

        // STUN only — server doesn't need TURN relay
        for url in &self.stun_urls {
            servers.push(IceServerConfig {
                urls: vec![url.clone()],
                username: None,
                credential: None,
            });
        }

        servers
    }

    /// Build the full ICE server list including TURN (for client API responses).
    pub fn ice_servers_with_turn(&self) -> Vec<IceServerConfig> {
        let mut servers = self.ice_servers_for_server();

        // TURN (embedded or external) — only for clients behind NAT
        if self.turn_embedded {
            let turn_url = format!(
                "turn:{}:{}",
                self.public_host, self.turn_port
            );
            servers.push(IceServerConfig {
                urls: vec![turn_url],
                username: Some(self.turn_username.clone()),
                credential: Some(self.turn_password.clone()),
            });
        } else {
            for url in &self.turn_urls {
                servers.push(IceServerConfig {
                    urls: vec![url.clone()],
                    username: Some(self.turn_username.clone()),
                    credential: Some(self.turn_password.clone()),
                });
            }
        }

        servers
    }

    /// Build the ICE server list to send to browser clients via the API.
    ///
    /// This is a JSON-serialisable format compatible with the W3C
    /// `RTCIceServer` dictionary.
    pub fn ice_servers_for_client(&self) -> Vec<ClientIceServer> {
        self.ice_servers_with_turn()
            .into_iter()
            .map(|s| ClientIceServer {
                urls: s.urls,
                username: s.username,
                credential: s.credential,
            })
            .collect()
    }

    fn log_summary(&self) {
        info!("──── LiveRelay Configuration ────");
        info!("  bind_addr          : {}", self.bind_addr);
        info!("  public_host        : {}", self.public_host);
        info!("  tls_enabled        : {}", self.tls_enabled);
        if self.tls_enabled {
            info!(
                "  tls_cert_path      : {}",
                self.tls_cert_path.as_deref().unwrap_or("(not set)")
            );
            info!(
                "  tls_key_path       : {}",
                self.tls_key_path.as_deref().unwrap_or("(not set)")
            );
        }
        info!("  turn_embedded      : {}", self.turn_embedded);
        if self.turn_embedded {
            info!("  turn_port          : {}", self.turn_port);
            info!("  turn_realm         : {}", self.turn_realm);
        }
        info!("  stun_urls          : {:?}", self.stun_urls);
        info!("  turn_urls          : {:?}", self.turn_urls);
        info!("  max_rooms          : {}", self.max_rooms);
        info!(
            "  max_subs_per_room  : {}",
            self.max_subscribers_per_room
        );
        info!(
            "  cors_origins       : {}",
            if self.allowed_origins == "*" {
                "* (permissive)"
            } else {
                &self.allowed_origins
            }
        );
        info!("  log_level          : {}", self.log_level);
        info!("────────────────────────────────");
    }
}

// ---------------------------------------------------------------------------
// ICE server configuration types
// ---------------------------------------------------------------------------

/// Internal ICE server representation (for both server and client).
#[derive(Debug, Clone)]
pub struct IceServerConfig {
    pub urls: Vec<String>,
    pub username: Option<String>,
    pub credential: Option<String>,
}

/// JSON-serialisable ICE server config sent to browser clients.
///
/// Matches the W3C `RTCIceServer` dictionary shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientIceServer {
    pub urls: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credential: Option<String>,
}

// ---------------------------------------------------------------------------
// Environment helpers
// ---------------------------------------------------------------------------

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

fn env_bool(key: &str, default: bool) -> bool {
    match std::env::var(key) {
        Ok(v) => matches!(v.to_lowercase().as_str(), "1" | "true" | "yes"),
        Err(_) => default,
    }
}

fn env_csv(key: &str, defaults: &[&str]) -> Vec<String> {
    match std::env::var(key) {
        Ok(v) if !v.is_empty() => v
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
        _ => defaults.iter().map(|s| s.to_string()).collect(),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_ice_servers_include_stun() {
        // Set minimal env
        std::env::remove_var("LIVERELAY_TURN_EMBEDDED");
        std::env::remove_var("LIVERELAY_TURN_URLS");
        std::env::remove_var("LIVERELAY_STUN_URLS");

        let config = Config {
            bind_addr: "0.0.0.0:8080".into(),
            public_host: "localhost".into(),
            tls_enabled: false,
            tls_cert_path: None,
            tls_key_path: None,
            turn_embedded: false,
            turn_port: 3478,
            turn_username: "test".into(),
            turn_password: "test".into(),
            turn_realm: "localhost".into(),
            stun_urls: vec!["stun:stun.l.google.com:19302".into()],
            turn_urls: vec![],
            jwt_secret: "test".into(),
            max_rooms: 100,
            max_subscribers_per_room: 1000,
            allowed_origins: "*".into(),
            log_level: "info".into(),
        };

        let servers = config.ice_servers_for_server();
        assert!(!servers.is_empty());
        assert!(servers[0].urls[0].starts_with("stun:"));
    }

    #[test]
    fn embedded_turn_generates_url() {
        let config = Config {
            bind_addr: "0.0.0.0:8080".into(),
            public_host: "sfu.example.com".into(),
            tls_enabled: false,
            tls_cert_path: None,
            tls_key_path: None,
            turn_embedded: true,
            turn_port: 3478,
            turn_username: "user".into(),
            turn_password: "pass".into(),
            turn_realm: "sfu.example.com".into(),
            stun_urls: vec![],
            turn_urls: vec![],
            jwt_secret: "test".into(),
            max_rooms: 100,
            max_subscribers_per_room: 1000,
            allowed_origins: "*".into(),
            log_level: "info".into(),
        };

        let servers = config.ice_servers_for_server();
        let turn_server = servers
            .iter()
            .find(|s| s.urls[0].starts_with("turn:"))
            .expect("expected a TURN server entry");

        assert_eq!(turn_server.urls[0], "turn:sfu.example.com:3478");
        assert_eq!(turn_server.username.as_deref(), Some("user"));
        assert_eq!(turn_server.credential.as_deref(), Some("pass"));
    }

    #[test]
    fn client_ice_servers_serializes() {
        let server = ClientIceServer {
            urls: vec!["turn:example.com:3478".into()],
            username: Some("user".into()),
            credential: Some("pass".into()),
        };
        let json = serde_json::to_string(&server).unwrap();
        assert!(json.contains("turn:example.com:3478"));
        assert!(json.contains("\"username\""));
    }
}
