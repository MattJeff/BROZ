// src/sse.rs
//
// Server-Sent Events (SSE) endpoint for LiveRelay.
//
// ─ Usage ────────────────────────────────────────────────────────────────────
//
//   GET /v1/events?room_id=<room_id>
//   Authorization: Bearer lr_...
//
//   The connection stays open and streams events as they occur in real-time.
//
//   Optional query parameters:
//     room_id   -- filter events to a specific room (omit for all rooms).
//     types     -- comma-separated event types to receive
//                  (e.g. "participant.joined,participant.left").
//
//   Each SSE message has:
//     event: <event_type>       (e.g. "participant.joined")
//     id:    <event_id>         (e.g. "evt_a1b2c3d4")
//     data:  <json payload>
//
// ─ Implementation ───────────────────────────────────────────────────────────
//
//   The handler subscribes to the `EventBus` broadcast channel and converts
//   each received event into an SSE frame.  Filtering by room_id and event
//   type is done in the stream itself so only matching events are sent over
//   the wire.
//
// ────────────────────────────────────────────────────────────────────────────

use axum::{
    extract::{Query, State},
    http::HeaderMap,
    response::sse::{Event as SseEvent, KeepAlive, Sse},
};
use futures::stream::Stream;
use serde::Deserialize;
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, warn};

use crate::events::{EventType, LiveRelayEvent};

// ─── Query parameters ───────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct SseQuery {
    /// Filter to a specific room.
    pub room_id: Option<String>,

    /// Comma-separated list of event types.  Example: "participant.joined,room.deleted"
    pub types: Option<String>,
}

impl SseQuery {
    /// Parse the `types` param into a set of `EventType`.
    fn parsed_types(&self) -> Option<Vec<EventType>> {
        self.types.as_ref().map(|s| {
            s.split(',')
                .filter_map(|t| {
                    let trimmed = t.trim();
                    // Try deserializing the string as an EventType.
                    serde_json::from_str::<EventType>(&format!("\"{trimmed}\"")).ok()
                })
                .collect()
        })
    }

    /// Returns `true` if the event matches this query's filters.
    fn matches(&self, event: &LiveRelayEvent) -> bool {
        // Room filter.
        if let Some(ref room_id) = self.room_id {
            if event.room_id() != room_id {
                return false;
            }
        }

        // Type filter.
        if let Some(types) = self.parsed_types() {
            if !types.is_empty() && !types.contains(&event.event_type) {
                return false;
            }
        }

        true
    }
}

// ─── SSE handler ────────────────────────────────────────────────────────────

/// `GET /v1/events` -- SSE stream of real-time events.
///
/// The stream emits a heartbeat comment every 15 seconds to keep the
/// connection alive through proxies and load balancers.
pub async fn sse_events(
    State(state): State<Arc<crate::AppState>>,
    headers: HeaderMap,
    Query(query): Query<SseQuery>,
) -> Result<Sse<impl Stream<Item = Result<SseEvent, Infallible>>>, crate::error::ApiError> {
    // Require API key authentication.
    crate::auth::require_api_key(&headers, &state.api_keys).await?;

    let mut rx = state.event_bus.subscribe();

    info!(
        room_id = query.room_id.as_deref().unwrap_or("*"),
        types = query.types.as_deref().unwrap_or("*"),
        "SSE client connected"
    );

    let stream = async_stream::stream! {
        loop {
            match rx.recv().await {
                Ok(event) => {
                    if !query.matches(&event) {
                        continue;
                    }

                    let json = match serde_json::to_string(&event) {
                        Ok(j) => j,
                        Err(e) => {
                            warn!("SSE: failed to serialize event: {e}");
                            continue;
                        }
                    };

                    let sse_event = SseEvent::default()
                        .event(event.event_type.as_str())
                        .id(event.id.clone())
                        .data(json);

                    yield Ok(sse_event);
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    warn!("SSE client lagged, skipped {n} events");
                    // Send a warning event so the client knows it missed data.
                    let warning = SseEvent::default()
                        .event("_warning")
                        .data(format!("{{\"message\":\"lagged, skipped {n} events\"}}"));
                    yield Ok(warning);
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    info!("SSE: event bus closed, ending stream");
                    break;
                }
            }
        }
    };

    Ok(Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("heartbeat"),
    ))
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::LiveRelayEvent;

    #[test]
    fn query_matches_no_filter() {
        let query = SseQuery {
            room_id: None,
            types: None,
        };
        let evt = LiveRelayEvent::room_created("room-1", "broadcast");
        assert!(query.matches(&evt));
    }

    #[test]
    fn query_matches_room_filter() {
        let query = SseQuery {
            room_id: Some("room-1".to_string()),
            types: None,
        };
        let evt1 = LiveRelayEvent::room_created("room-1", "broadcast");
        let evt2 = LiveRelayEvent::room_created("room-2", "broadcast");
        assert!(query.matches(&evt1));
        assert!(!query.matches(&evt2));
    }

    #[test]
    fn query_matches_type_filter() {
        let query = SseQuery {
            room_id: None,
            types: Some("participant.joined,participant.left".to_string()),
        };
        let evt1 = LiveRelayEvent::participant_joined("r", "p", "publish");
        let evt2 = LiveRelayEvent::room_created("r", "broadcast");
        assert!(query.matches(&evt1));
        assert!(!query.matches(&evt2));
    }

    #[test]
    fn query_matches_combined_filters() {
        let query = SseQuery {
            room_id: Some("room-X".to_string()),
            types: Some("stream.started".to_string()),
        };
        let good = LiveRelayEvent::stream_started("room-X", "p1", "video");
        let wrong_room = LiveRelayEvent::stream_started("room-Y", "p1", "video");
        let wrong_type = LiveRelayEvent::participant_joined("room-X", "p1", "publish");

        assert!(query.matches(&good));
        assert!(!query.matches(&wrong_room));
        assert!(!query.matches(&wrong_type));
    }
}
