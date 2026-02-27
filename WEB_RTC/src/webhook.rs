// src/webhook.rs
//
// Webhook delivery engine for LiveRelay.
//
// ─ Architecture ─────────────────────────────────────────────────────────────
//
//   EventBus ──subscribe()──> WebhookDispatcher (background task)
//                                  │
//                                  ├─ filter by event type
//                                  ├─ sign payload (HMAC-SHA256)
//                                  ├─ POST to endpoint
//                                  └─ retry with exponential backoff
//
// ─ Security ─────────────────────────────────────────────────────────────────
//
//   Every outgoing POST carries two headers:
//     X-LiveRelay-Signature:  HMAC-SHA256(secret, body)
//     X-LiveRelay-Timestamp:  Unix epoch seconds
//
//   The recipient should reject requests older than 5 minutes (replay window).
//
// ─ Retry policy ─────────────────────────────────────────────────────────────
//
//   Attempt 1: immediate
//   Attempt 2: 1 s   (base_delay * 2^0)
//   Attempt 3: 2 s   (base_delay * 2^1)
//   Attempt 4: 4 s   (base_delay * 2^2)
//   Attempt 5: 8 s   (base_delay * 2^3)
//   Then give up and log the failure.
//
// ────────────────────────────────────────────────────────────────────────────

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use hmac::{Hmac, Mac};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

use crate::events::{EventBus, EventType, LiveRelayEvent};

// ─── Configuration structures ───────────────────────────────────────────────

/// A registered webhook endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    /// Unique ID (`wh_<uuid>`).
    pub id: String,

    /// Target URL.  Must be HTTPS in production.
    pub url: String,

    /// Shared secret for HMAC-SHA256 signing.
    ///
    /// Generated server-side and returned **once** at creation time.
    pub secret: String,

    /// Optional filter: if non-empty, only matching event types are delivered.
    /// An empty vec means "subscribe to everything".
    #[serde(default)]
    pub events: Vec<EventType>,

    /// Whether this webhook is currently active.
    #[serde(default = "default_true")]
    pub active: bool,

    /// Unix timestamp of creation.
    pub created_at: u64,
}

fn default_true() -> bool {
    true
}

impl WebhookConfig {
    /// Returns `true` if this webhook should receive the given event type.
    pub fn accepts(&self, event_type: &EventType) -> bool {
        self.active && (self.events.is_empty() || self.events.contains(event_type))
    }
}

// ─── Webhook store ──────────────────────────────────────────────────────────

/// Thread-safe store of webhook registrations.
///
/// Keyed by webhook `id`.
#[derive(Clone, Default)]
pub struct WebhookStore {
    inner: Arc<RwLock<HashMap<String, WebhookConfig>>>,
}

impl WebhookStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn insert(&self, config: WebhookConfig) {
        let mut map = self.inner.write().await;
        map.insert(config.id.clone(), config);
    }

    pub async fn get(&self, id: &str) -> Option<WebhookConfig> {
        let map = self.inner.read().await;
        map.get(id).cloned()
    }

    pub async fn list(&self) -> Vec<WebhookConfig> {
        let map = self.inner.read().await;
        map.values().cloned().collect()
    }

    pub async fn remove(&self, id: &str) -> Option<WebhookConfig> {
        let mut map = self.inner.write().await;
        map.remove(id)
    }

    /// Return all *active* webhooks that accept the given event type.
    pub async fn matching(&self, event_type: &EventType) -> Vec<WebhookConfig> {
        let map = self.inner.read().await;
        map.values()
            .filter(|wh| wh.accepts(event_type))
            .cloned()
            .collect()
    }
}

// ─── HMAC signing ───────────────────────────────────────────────────────────

type HmacSha256 = Hmac<Sha256>;

/// Compute the HMAC-SHA256 signature for a webhook delivery.
///
/// The signed message is `{timestamp}.{body}` (timestamp-prefixed to prevent
/// replay attacks).
pub fn sign_payload(secret: &str, timestamp: u64, body: &[u8]) -> String {
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC accepts any key length");
    mac.update(format!("{timestamp}.").as_bytes());
    mac.update(body);
    let result = mac.finalize();
    hex::encode(result.into_bytes())
}

/// Verify a received signature.
#[allow(dead_code)]
pub fn verify_signature(secret: &str, timestamp: u64, body: &[u8], signature: &str) -> bool {
    let expected = sign_payload(secret, timestamp, body);
    // Constant-time comparison.
    use subtle::ConstantTimeEq;
    expected.as_bytes().ct_eq(signature.as_bytes()).into()
}

// ─── Delivery with retry ────────────────────────────────────────────────────

/// Retry configuration.
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Maximum number of attempts (including the first).
    pub max_attempts: u32,
    /// Base delay before the first retry.
    pub base_delay: Duration,
    /// Maximum delay cap.
    pub max_delay: Duration,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 5,
            base_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(30),
        }
    }
}

impl RetryPolicy {
    /// Compute the delay for attempt `n` (0-indexed).
    fn delay_for(&self, attempt: u32) -> Duration {
        let delay = self.base_delay * 2u32.saturating_pow(attempt);
        delay.min(self.max_delay)
    }
}

/// Delivery result, useful for logging / metrics.
#[derive(Debug)]
#[allow(dead_code)]
pub struct DeliveryResult {
    pub webhook_id: String,
    pub event_id: String,
    pub attempts: u32,
    pub success: bool,
    pub last_status: Option<u16>,
    pub last_error: Option<String>,
}

/// Deliver an event to a single webhook endpoint with retries.
async fn deliver(
    client: &Client,
    webhook: &WebhookConfig,
    event: &LiveRelayEvent,
    policy: &RetryPolicy,
) -> DeliveryResult {
    let body = serde_json::to_vec(event).expect("event serialization cannot fail");
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let signature = sign_payload(&webhook.secret, timestamp, &body);

    let mut last_status = None;
    let mut last_error = None;

    for attempt in 0..policy.max_attempts {
        if attempt > 0 {
            let delay = policy.delay_for(attempt - 1);
            info!(
                webhook_id = %webhook.id,
                attempt = attempt + 1,
                delay_ms = delay.as_millis() as u64,
                "retrying webhook delivery"
            );
            tokio::time::sleep(delay).await;
        }

        let result = client
            .post(&webhook.url)
            .header("Content-Type", "application/json")
            .header("X-LiveRelay-Signature", &signature)
            .header("X-LiveRelay-Timestamp", timestamp.to_string())
            .header("X-LiveRelay-Event", event.event_type.as_str())
            .header("X-LiveRelay-Delivery", &event.id)
            .header("User-Agent", "LiveRelay-Webhook/0.2.0")
            .body(body.clone())
            .timeout(Duration::from_secs(10))
            .send()
            .await;

        match result {
            Ok(resp) => {
                let status = resp.status().as_u16();
                last_status = Some(status);

                if (200..300).contains(&(status as usize)) {
                    info!(
                        webhook_id = %webhook.id,
                        event_id = %event.id,
                        status,
                        attempts = attempt + 1,
                        "webhook delivered"
                    );
                    return DeliveryResult {
                        webhook_id: webhook.id.clone(),
                        event_id: event.id.clone(),
                        attempts: attempt + 1,
                        success: true,
                        last_status: Some(status),
                        last_error: None,
                    };
                }

                last_error = Some(format!("HTTP {status}"));
                warn!(
                    webhook_id = %webhook.id,
                    event_id = %event.id,
                    status,
                    attempt = attempt + 1,
                    "webhook delivery got non-2xx"
                );
            }
            Err(e) => {
                last_error = Some(e.to_string());
                warn!(
                    webhook_id = %webhook.id,
                    event_id = %event.id,
                    error = %e,
                    attempt = attempt + 1,
                    "webhook delivery failed"
                );
            }
        }
    }

    error!(
        webhook_id = %webhook.id,
        event_id = %event.id,
        "webhook delivery exhausted all {} attempts",
        policy.max_attempts
    );

    DeliveryResult {
        webhook_id: webhook.id.clone(),
        event_id: event.id.clone(),
        attempts: policy.max_attempts,
        success: false,
        last_status,
        last_error,
    }
}

// ─── Background dispatcher ──────────────────────────────────────────────────

/// Spawn the background task that reads events from the bus and fans them out
/// to every matching webhook endpoint.
///
/// Returns a `JoinHandle` so the caller can await or abort on shutdown.
pub fn spawn_webhook_dispatcher(
    bus: EventBus,
    store: WebhookStore,
    policy: RetryPolicy,
) -> tokio::task::JoinHandle<()> {
    let client = Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .expect("failed to build reqwest client");

    let mut rx = bus.subscribe();

    tokio::spawn(async move {
        info!("webhook dispatcher started");
        loop {
            match rx.recv().await {
                Ok(event) => {
                    let webhooks = store.matching(&event.event_type).await;
                    if webhooks.is_empty() {
                        continue;
                    }

                    // Fan-out: deliver to each matching webhook concurrently.
                    let client = client.clone();
                    let policy = policy.clone();
                    for wh in webhooks {
                        let client = client.clone();
                        let event = event.clone();
                        let policy = policy.clone();
                        tokio::spawn(async move {
                            deliver(&client, &wh, &event, &policy).await;
                        });
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    warn!("webhook dispatcher lagged, skipped {n} events");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    info!("webhook dispatcher shutting down (bus closed)");
                    break;
                }
            }
        }
    })
}

// ─── API DTOs ───────────────────────────────────────────────────────────────

/// Request body for `POST /v1/webhooks`.
#[derive(Debug, Deserialize)]
pub struct CreateWebhookRequest {
    /// Target URL.
    pub url: String,

    /// Optional event filter.  If omitted or empty, all events are delivered.
    #[serde(default)]
    pub events: Vec<EventType>,
}

/// Response body for `POST /v1/webhooks`.
///
/// The `secret` is only shown once -- the caller must store it.
#[derive(Debug, Serialize)]
pub struct CreateWebhookResponse {
    pub id: String,
    pub url: String,
    pub secret: String,
    pub events: Vec<EventType>,
    pub active: bool,
    pub created_at: u64,
}

/// Public view of a webhook (secret redacted).
#[derive(Debug, Serialize)]
pub struct WebhookView {
    pub id: String,
    pub url: String,
    pub events: Vec<EventType>,
    pub active: bool,
    pub created_at: u64,
}

impl From<WebhookConfig> for WebhookView {
    fn from(wh: WebhookConfig) -> Self {
        Self {
            id: wh.id,
            url: wh.url,
            events: wh.events,
            active: wh.active,
            created_at: wh.created_at,
        }
    }
}

// ─── Axum handlers ──────────────────────────────────────────────────────────

/// `POST /v1/webhooks` -- register a new webhook.
pub async fn create_webhook(
    State(state): State<Arc<crate::AppState>>,
    headers: HeaderMap,
    Json(body): Json<CreateWebhookRequest>,
) -> Result<(StatusCode, Json<CreateWebhookResponse>), crate::error::ApiError> {
    // Require API key.
    crate::auth::require_api_key(&headers, &state.api_keys).await?;

    // Validate URL.
    if !body.url.starts_with("http://") && !body.url.starts_with("https://") {
        return Err(crate::error::ApiError::bad_request(
            "Webhook URL must start with http:// or https://",
        ));
    }

    let id = format!("wh_{}", uuid::Uuid::new_v4());
    let secret = format!("whsec_{}", uuid::Uuid::new_v4());
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let config = WebhookConfig {
        id: id.clone(),
        url: body.url.clone(),
        secret: secret.clone(),
        events: body.events.clone(),
        active: true,
        created_at: now,
    };

    state.webhooks.insert(config).await;

    info!(webhook_id = %id, url = %body.url, "webhook registered");

    Ok((
        StatusCode::CREATED,
        Json(CreateWebhookResponse {
            id,
            url: body.url,
            secret,
            events: body.events,
            active: true,
            created_at: now,
        }),
    ))
}

/// `GET /v1/webhooks` -- list all webhooks (secrets redacted).
pub async fn list_webhooks(
    State(state): State<Arc<crate::AppState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<WebhookView>>, crate::error::ApiError> {
    crate::auth::require_api_key(&headers, &state.api_keys).await?;

    let all = state.webhooks.list().await;
    let views: Vec<WebhookView> = all.into_iter().map(WebhookView::from).collect();

    Ok(Json(views))
}

/// `DELETE /v1/webhooks/:id` -- unregister a webhook.
pub async fn delete_webhook(
    State(state): State<Arc<crate::AppState>>,
    Path(webhook_id): Path<String>,
    headers: HeaderMap,
) -> Result<StatusCode, crate::error::ApiError> {
    crate::auth::require_api_key(&headers, &state.api_keys).await?;

    match state.webhooks.remove(&webhook_id).await {
        Some(_) => {
            info!(webhook_id = %webhook_id, "webhook deleted");
            Ok(StatusCode::NO_CONTENT)
        }
        None => Err(crate::error::ApiError::not_found(format!(
            "Webhook '{webhook_id}' not found."
        ))),
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hmac_roundtrip() {
        let secret = "whsec_test_secret";
        let body = b"{\"type\":\"room.created\"}";
        let ts = 1718000000u64;

        let sig = sign_payload(secret, ts, body);
        assert!(verify_signature(secret, ts, body, &sig));
        assert!(!verify_signature("wrong_secret", ts, body, &sig));
        assert!(!verify_signature(secret, ts + 1, body, &sig));
    }

    #[test]
    fn webhook_filter_accepts_all() {
        let wh = WebhookConfig {
            id: "wh_1".into(),
            url: "https://example.com/hook".into(),
            secret: "s".into(),
            events: vec![], // empty = all
            active: true,
            created_at: 0,
        };
        assert!(wh.accepts(&EventType::RoomCreated));
        assert!(wh.accepts(&EventType::QualityDegraded));
    }

    #[test]
    fn webhook_filter_specific() {
        let wh = WebhookConfig {
            id: "wh_2".into(),
            url: "https://example.com/hook".into(),
            secret: "s".into(),
            events: vec![EventType::ParticipantJoined, EventType::ParticipantLeft],
            active: true,
            created_at: 0,
        };
        assert!(wh.accepts(&EventType::ParticipantJoined));
        assert!(!wh.accepts(&EventType::RoomCreated));
    }

    #[test]
    fn inactive_webhook_rejects() {
        let wh = WebhookConfig {
            id: "wh_3".into(),
            url: "https://example.com/hook".into(),
            secret: "s".into(),
            events: vec![],
            active: false,
            created_at: 0,
        };
        assert!(!wh.accepts(&EventType::RoomCreated));
    }

    #[test]
    fn retry_policy_backoff() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.delay_for(0), Duration::from_secs(1));
        assert_eq!(policy.delay_for(1), Duration::from_secs(2));
        assert_eq!(policy.delay_for(2), Duration::from_secs(4));
        assert_eq!(policy.delay_for(3), Duration::from_secs(8));
        // Capped at max_delay (30s).
        assert_eq!(policy.delay_for(10), Duration::from_secs(30));
    }
}
