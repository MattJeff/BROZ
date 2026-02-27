// src/events.rs
//
// Central event bus for LiveRelay.
//
// Every meaningful state change (room lifecycle, participant lifecycle, stream
// lifecycle, quality changes) is represented as a `LiveRelayEvent`.  A single
// `EventBus` backed by a `tokio::sync::broadcast` channel fans out each event
// to every consumer: the webhook dispatcher, the SSE stream, the analytics
// collector, and (optionally) the SDK DataChannel bridge.
//
// ────────────────────────────────────────────────────────────────────────────

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tracing::debug;

// ─── Event types ────────────────────────────────────────────────────────────

/// Canonical event type string, used in JSON payloads and webhook filters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EventType {
    #[serde(rename = "room.created")]
    RoomCreated,
    #[serde(rename = "room.deleted")]
    RoomDeleted,
    #[serde(rename = "participant.joined")]
    ParticipantJoined,
    #[serde(rename = "participant.left")]
    ParticipantLeft,
    #[serde(rename = "stream.started")]
    StreamStarted,
    #[serde(rename = "stream.stopped")]
    StreamStopped,
    #[serde(rename = "quality.degraded")]
    QualityDegraded,
}

impl EventType {
    /// Stable string representation used in HTTP headers, SSE `event:` fields,
    /// and filter expressions.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::RoomCreated => "room.created",
            Self::RoomDeleted => "room.deleted",
            Self::ParticipantJoined => "participant.joined",
            Self::ParticipantLeft => "participant.left",
            Self::StreamStarted => "stream.started",
            Self::StreamStopped => "stream.stopped",
            Self::QualityDegraded => "quality.degraded",
        }
    }
}

impl std::fmt::Display for EventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ─── Event payloads ─────────────────────────────────────────────────────────

/// Metadata attached to room lifecycle events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomPayload {
    pub room_id: String,
    pub room_type: String,
}

/// Metadata attached to participant lifecycle events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticipantPayload {
    pub room_id: String,
    pub peer_id: String,
    pub role: String,
}

/// Metadata attached to stream lifecycle events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamPayload {
    pub room_id: String,
    pub peer_id: String,
    pub kind: String, // "audio" | "video" | "audio+video"
}

/// Metadata attached to quality degradation events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityPayload {
    pub room_id: String,
    pub peer_id: String,
    pub metric: String,       // e.g. "packet_loss", "latency", "mos"
    pub value: f64,
    pub threshold: f64,
    pub direction: String,    // "above" | "below"
}

/// Type-safe union of all possible payloads.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EventPayload {
    Room(RoomPayload),
    Participant(ParticipantPayload),
    Stream(StreamPayload),
    Quality(QualityPayload),
}

// ─── The event envelope ─────────────────────────────────────────────────────

/// A fully self-describing event, ready for serialisation.
///
/// ```json
/// {
///   "id":         "evt_a1b2c3d4",
///   "type":       "participant.joined",
///   "created_at": "2025-06-15T14:22:33.123Z",
///   "data": {
///     "room_id": "...",
///     "peer_id": "...",
///     "role":    "publish"
///   }
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveRelayEvent {
    /// Globally unique event identifier (format: `evt_<uuid-v4>`).
    pub id: String,

    /// Event type.
    #[serde(rename = "type")]
    pub event_type: EventType,

    /// ISO-8601 timestamp (UTC).
    pub created_at: DateTime<Utc>,

    /// Type-specific payload.
    pub data: EventPayload,
}

impl LiveRelayEvent {
    // ── Constructors ────────────────────────────────────────────────────

    /// Build a `room.created` event.
    pub fn room_created(room_id: &str, room_type: &str) -> Self {
        Self::new(
            EventType::RoomCreated,
            EventPayload::Room(RoomPayload {
                room_id: room_id.to_string(),
                room_type: room_type.to_string(),
            }),
        )
    }

    /// Build a `room.deleted` event.
    pub fn room_deleted(room_id: &str, room_type: &str) -> Self {
        Self::new(
            EventType::RoomDeleted,
            EventPayload::Room(RoomPayload {
                room_id: room_id.to_string(),
                room_type: room_type.to_string(),
            }),
        )
    }

    /// Build a `participant.joined` event.
    pub fn participant_joined(room_id: &str, peer_id: &str, role: &str) -> Self {
        Self::new(
            EventType::ParticipantJoined,
            EventPayload::Participant(ParticipantPayload {
                room_id: room_id.to_string(),
                peer_id: peer_id.to_string(),
                role: role.to_string(),
            }),
        )
    }

    /// Build a `participant.left` event.
    pub fn participant_left(room_id: &str, peer_id: &str, role: &str) -> Self {
        Self::new(
            EventType::ParticipantLeft,
            EventPayload::Participant(ParticipantPayload {
                room_id: room_id.to_string(),
                peer_id: peer_id.to_string(),
                role: role.to_string(),
            }),
        )
    }

    /// Build a `stream.started` event.
    pub fn stream_started(room_id: &str, peer_id: &str, kind: &str) -> Self {
        Self::new(
            EventType::StreamStarted,
            EventPayload::Stream(StreamPayload {
                room_id: room_id.to_string(),
                peer_id: peer_id.to_string(),
                kind: kind.to_string(),
            }),
        )
    }

    /// Build a `stream.stopped` event.
    pub fn stream_stopped(room_id: &str, peer_id: &str, kind: &str) -> Self {
        Self::new(
            EventType::StreamStopped,
            EventPayload::Stream(StreamPayload {
                room_id: room_id.to_string(),
                peer_id: peer_id.to_string(),
                kind: kind.to_string(),
            }),
        )
    }

    /// Build a `quality.degraded` event.
    pub fn quality_degraded(
        room_id: &str,
        peer_id: &str,
        metric: &str,
        value: f64,
        threshold: f64,
        direction: &str,
    ) -> Self {
        Self::new(
            EventType::QualityDegraded,
            EventPayload::Quality(QualityPayload {
                room_id: room_id.to_string(),
                peer_id: peer_id.to_string(),
                metric: metric.to_string(),
                value,
                threshold,
                direction: direction.to_string(),
            }),
        )
    }

    // ── Private ─────────────────────────────────────────────────────────

    fn new(event_type: EventType, data: EventPayload) -> Self {
        Self {
            id: format!("evt_{}", uuid::Uuid::new_v4()),
            event_type,
            created_at: Utc::now(),
            data,
        }
    }

    /// Extract the `room_id` from any payload variant.
    pub fn room_id(&self) -> &str {
        match &self.data {
            EventPayload::Room(p) => &p.room_id,
            EventPayload::Participant(p) => &p.room_id,
            EventPayload::Stream(p) => &p.room_id,
            EventPayload::Quality(p) => &p.room_id,
        }
    }
}

// ─── EventBus ───────────────────────────────────────────────────────────────

/// Broadcast-based fan-out channel for `LiveRelayEvent`.
///
/// Capacity is generous (4096 events) -- subscribers that lag more than that
/// will skip events (same semantic as `broadcast::RecvError::Lagged`).
///
/// The bus is **cheap to clone** (interior `Arc`).
#[derive(Clone)]
pub struct EventBus {
    tx: broadcast::Sender<LiveRelayEvent>,
}

impl EventBus {
    /// Create a new bus with the default capacity.
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(4096);
        Self { tx }
    }

    /// Create a new bus with a custom capacity.
    pub fn with_capacity(cap: usize) -> Self {
        let (tx, _) = broadcast::channel(cap);
        Self { tx }
    }

    /// Publish an event.  Returns the number of active subscribers that will
    /// receive it.  Silently succeeds even if there are no subscribers.
    pub fn emit(&self, event: LiveRelayEvent) -> usize {
        debug!(event_type = %event.event_type, event_id = %event.id, "event emitted");
        // broadcast::send returns Err only if there are 0 receivers, which is
        // perfectly normal during startup or if no SSE / webhook is connected.
        self.tx.send(event).unwrap_or(0)
    }

    /// Obtain a new receiver.  Each receiver gets an independent copy of every
    /// event published *after* this call.
    pub fn subscribe(&self) -> broadcast::Receiver<LiveRelayEvent> {
        self.tx.subscribe()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_type_serialization() {
        let json = serde_json::to_string(&EventType::ParticipantJoined).unwrap();
        assert_eq!(json, "\"participant.joined\"");

        let parsed: EventType = serde_json::from_str("\"room.created\"").unwrap();
        assert_eq!(parsed, EventType::RoomCreated);
    }

    #[test]
    fn event_envelope_json() {
        let evt = LiveRelayEvent::room_created("room-1", "broadcast");
        let json = serde_json::to_string_pretty(&evt).unwrap();
        assert!(json.contains("\"type\": \"room.created\""));
        assert!(json.contains("\"room_id\": \"room-1\""));
        assert!(evt.id.starts_with("evt_"));
    }

    #[tokio::test]
    async fn bus_fanout() {
        let bus = EventBus::new();
        let mut rx1 = bus.subscribe();
        let mut rx2 = bus.subscribe();

        let evt = LiveRelayEvent::room_created("r1", "broadcast");
        let n = bus.emit(evt.clone());
        assert_eq!(n, 2);

        let e1 = rx1.recv().await.unwrap();
        let e2 = rx2.recv().await.unwrap();
        assert_eq!(e1.id, e2.id);
    }

    #[test]
    fn room_id_extraction() {
        let e = LiveRelayEvent::participant_joined("room-42", "peer-7", "publish");
        assert_eq!(e.room_id(), "room-42");

        let e = LiveRelayEvent::quality_degraded("room-99", "peer-3", "packet_loss", 12.5, 5.0, "above");
        assert_eq!(e.room_id(), "room-99");
    }
}
