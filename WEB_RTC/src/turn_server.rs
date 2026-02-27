//! Embedded TURN server using the `turn` crate (part of the webrtc-rs ecosystem).
//!
//! This module provides two options:
//!
//! 1. **Embedded TURN** -- a TURN server running inside the same binary as the
//!    SFU, started as a background tokio task.  No external process needed.
//!
//! 2. **External TURN** (coturn etc.) -- the SFU simply advertises the external
//!    TURN URLs / credentials to clients via the ICE config API endpoint.
//!
//! The choice is driven by `Config::turn_embedded`.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use tokio::net::UdpSocket;
use tracing::info;
use turn::auth::*;
use turn::relay::relay_static::*;
use util::vnet::net::Net;
use turn::server::{config::*, *};
use turn::Error;

use crate::config::Config;

// ---------------------------------------------------------------------------
// Credential provider — long-term credentials (static username/password)
// ---------------------------------------------------------------------------

/// Simple long-term credential handler.
///
/// For production with many users you would generate per-session TURN
/// credentials (HMAC-based ephemeral passwords).  This implementation
/// uses a single shared username/password pair which is sufficient for a
/// single-tenant SFU.
struct StaticAuthHandler {
    credentials: HashMap<String, Vec<u8>>,
}

impl StaticAuthHandler {
    fn new(username: &str, password: &str, realm: &str) -> Self {
        let mut creds = HashMap::new();
        // TURN long-term credentials: key = MD5(username:realm:password)
        let key = generate_auth_key(username, realm, password);
        creds.insert(username.to_string(), key);
        Self {
            credentials: creds,
        }
    }
}

impl AuthHandler for StaticAuthHandler {
    fn auth_handle(
        &self,
        username: &str,
        _realm: &str,
        _src_addr: SocketAddr,
    ) -> Result<Vec<u8>, Error> {
        self.credentials
            .get(username)
            .cloned()
            .ok_or_else(|| Error::Other("unknown user".into()))
    }
}

// ---------------------------------------------------------------------------
// Generate TURN auth key (MD5-based, per RFC 5389)
// ---------------------------------------------------------------------------

fn generate_auth_key(username: &str, realm: &str, password: &str) -> Vec<u8> {
    let input = format!("{username}:{realm}:{password}");
    let digest = md5::compute(input.as_bytes());
    digest.to_vec()
}

/// Minimal MD5 implementation (TURN requires it for long-term credentials).
///
/// We avoid pulling in a full `md5` crate by implementing the bare minimum
/// here.  If the `md5` crate is already in the dependency tree (it often is
/// via `turn`), you could use it directly instead.
mod md5 {
    pub struct Digest([u8; 16]);

    impl Digest {
        pub fn to_vec(&self) -> Vec<u8> {
            self.0.to_vec()
        }
    }

    /// Compute the MD5 hash of the input bytes.
    ///
    /// This is a straightforward implementation of RFC 1321.
    pub fn compute(data: &[u8]) -> Digest {
        // Constants
        const S: [u32; 64] = [
            7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22,
            5,  9, 14, 20, 5,  9, 14, 20, 5,  9, 14, 20, 5,  9, 14, 20,
            4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23,
            6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
        ];

        const K: [u32; 64] = [
            0xd76aa478, 0xe8c7b756, 0x242070db, 0xc1bdceee,
            0xf57c0faf, 0x4787c62a, 0xa8304613, 0xfd469501,
            0x698098d8, 0x8b44f7af, 0xffff5bb1, 0x895cd7be,
            0x6b901122, 0xfd987193, 0xa679438e, 0x49b40821,
            0xf61e2562, 0xc040b340, 0x265e5a51, 0xe9b6c7aa,
            0xd62f105d, 0x02441453, 0xd8a1e681, 0xe7d3fbc8,
            0x21e1cde6, 0xc33707d6, 0xf4d50d87, 0x455a14ed,
            0xa9e3e905, 0xfcefa3f8, 0x676f02d9, 0x8d2a4c8a,
            0xfffa3942, 0x8771f681, 0x6d9d6122, 0xfde5380c,
            0xa4beea44, 0x4bdecfa9, 0xf6bb4b60, 0xbebfbc70,
            0x289b7ec6, 0xeaa127fa, 0xd4ef3085, 0x04881d05,
            0xd9d4d039, 0xe6db99e5, 0x1fa27cf8, 0xc4ac5665,
            0xf4292244, 0x432aff97, 0xab9423a7, 0xfc93a039,
            0x655b59c3, 0x8f0ccc92, 0xffeff47d, 0x85845dd1,
            0x6fa87e4f, 0xfe2ce6e0, 0xa3014314, 0x4e0811a1,
            0xf7537e82, 0xbd3af235, 0x2ad7d2bb, 0xeb86d391,
        ];

        // Padding
        let orig_len_bits = (data.len() as u64) * 8;
        let mut msg = data.to_vec();
        msg.push(0x80);
        while msg.len() % 64 != 56 {
            msg.push(0);
        }
        msg.extend_from_slice(&orig_len_bits.to_le_bytes());

        let mut a0: u32 = 0x67452301;
        let mut b0: u32 = 0xefcdab89;
        let mut c0: u32 = 0x98badcfe;
        let mut d0: u32 = 0x10325476;

        for chunk in msg.chunks_exact(64) {
            let mut m = [0u32; 16];
            for (i, word) in chunk.chunks_exact(4).enumerate() {
                m[i] = u32::from_le_bytes([word[0], word[1], word[2], word[3]]);
            }

            let (mut a, mut b, mut c, mut d) = (a0, b0, c0, d0);

            for i in 0..64 {
                let (f, g) = match i {
                    0..=15  => ((b & c) | ((!b) & d), i),
                    16..=31 => ((d & b) | ((!d) & c), (5 * i + 1) % 16),
                    32..=47 => (b ^ c ^ d, (3 * i + 5) % 16),
                    _       => (c ^ (b | (!d)), (7 * i) % 16),
                };

                let f = f.wrapping_add(a).wrapping_add(K[i]).wrapping_add(m[g]);
                a = d;
                d = c;
                c = b;
                b = b.wrapping_add(f.rotate_left(S[i]));
            }

            a0 = a0.wrapping_add(a);
            b0 = b0.wrapping_add(b);
            c0 = c0.wrapping_add(c);
            d0 = d0.wrapping_add(d);
        }

        let mut result = [0u8; 16];
        result[0..4].copy_from_slice(&a0.to_le_bytes());
        result[4..8].copy_from_slice(&b0.to_le_bytes());
        result[8..12].copy_from_slice(&c0.to_le_bytes());
        result[12..16].copy_from_slice(&d0.to_le_bytes());

        Digest(result)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn md5_empty() {
            let d = compute(b"");
            assert_eq!(
                hex(&d.0),
                "d41d8cd98f00b204e9800998ecf8427e"
            );
        }

        #[test]
        fn md5_hello() {
            let d = compute(b"hello");
            assert_eq!(
                hex(&d.0),
                "5d41402abc4b2a76b9719d911017c592"
            );
        }

        fn hex(bytes: &[u8]) -> String {
            bytes.iter().map(|b| format!("{:02x}", b)).collect()
        }
    }
}

// ---------------------------------------------------------------------------
// Start embedded TURN server
// ---------------------------------------------------------------------------

/// Spawn an embedded TURN relay server on the configured UDP port.
///
/// This runs as a background tokio task and returns a handle that can be
/// used to shut it down (by dropping or calling `close()`).
///
/// The TURN server listens on `0.0.0.0:<turn_port>` for UDP and
/// allocates relay addresses on the same interface.
pub async fn start_embedded_turn(config: &Config) -> Result<Arc<Server>, Box<dyn std::error::Error>> {
    let listen_addr = format!("0.0.0.0:{}", config.turn_port);
    let conn = Arc::new(UdpSocket::bind(&listen_addr).await?);

    info!(
        "Embedded TURN server binding to UDP {}  realm='{}'  user='{}'",
        listen_addr, config.turn_realm, config.turn_username
    );

    let auth_handler = Arc::new(StaticAuthHandler::new(
        &config.turn_username,
        &config.turn_password,
        &config.turn_realm,
    ));

    let server = Server::new(ServerConfig {
        conn_configs: vec![ConnConfig {
            conn,
            relay_addr_generator: Box::new(RelayAddressGeneratorStatic {
                relay_address: std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED),
                address: "0.0.0.0".to_string(),
                net: Arc::new(Net::new(None)),
            }),
        }],
        realm: config.turn_realm.clone(),
        auth_handler,
        channel_bind_timeout: std::time::Duration::from_secs(600),
        alloc_close_notify: None,
    })
    .await?;

    info!("Embedded TURN server started on {}", listen_addr);
    Ok(Arc::new(server))
}

// ---------------------------------------------------------------------------
// API endpoint: GET /v1/ice-servers
// ---------------------------------------------------------------------------

/// Returns the ICE server configuration that browser clients need.
///
/// Clients call this endpoint and use the response to configure their
/// `RTCPeerConnection`:
///
/// ```js
/// const resp = await fetch('/v1/ice-servers', {
///     headers: { 'Authorization': 'Bearer <jwt>' }
/// });
/// const { ice_servers } = await resp.json();
/// const pc = new RTCPeerConnection({ iceServers: ice_servers });
/// ```
pub async fn get_ice_servers(
    axum::extract::State(state): axum::extract::State<Arc<crate::AppState>>,
    headers: axum::http::HeaderMap,
) -> Result<axum::Json<serde_json::Value>, crate::error::ApiError> {
    // Require at least a valid JWT (any role) -- we do not want to expose
    // TURN credentials to unauthenticated callers.
    let token_str = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(crate::error::ApiError::auth_header_missing)?;

    crate::auth::verify_token(&state.jwt_secret, token_str).map_err(|e| {
        match e.kind() {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => {
                crate::error::ApiError::token_expired()
            }
            _ => crate::error::ApiError::token_invalid(),
        }
    })?;

    let ice_servers = state.config.ice_servers_for_client();

    Ok(axum::Json(serde_json::json!({
        "ice_servers": ice_servers
    })))
}
