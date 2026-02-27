use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::broadcast;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::rtp_transceiver::rtp_codec::RTCRtpCodecCapability;

// ---------------------------------------------------------------------------
// TrackSource — distinguishes camera from screen share
// ---------------------------------------------------------------------------

/// Label attached to a publisher's video track to indicate its source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TrackSource {
    Camera,
    Screen,
}

// ---------------------------------------------------------------------------
// RoomType
// ---------------------------------------------------------------------------

/// Describes the topology of a room.
///
/// * `Broadcast`  -- one publisher streams to N subscribers.
/// * `Call`       -- two peers, each acting as publisher *and* subscriber.
/// * `Conference` -- N peers, each publishes + subscribes to all others.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RoomType {
    /// 1 publisher -> N subscribers
    Broadcast,
    /// 2 publishers <-> 2 subscribers (each peer is both)
    Call,
    /// N publishers <-> N subscribers (each peer is both)
    Conference,
}

// ---------------------------------------------------------------------------
// Publisher
// ---------------------------------------------------------------------------

/// A publisher is a peer that sends media into a room.
///
/// Video and audio RTP packets are forwarded through broadcast channels so
/// that every subscriber can receive an independent copy.
///
/// Screen sharing uses a separate broadcast channel (`screen_tx`) so
/// subscribers can distinguish camera video from screen content.
pub struct Publisher {
    pub peer_id: String,
    pub pc: Arc<RTCPeerConnection>,

    // Camera video + audio
    pub video_tx: broadcast::Sender<webrtc::rtp::packet::Packet>,
    pub audio_tx: broadcast::Sender<webrtc::rtp::packet::Packet>,
    pub video_ssrc: AtomicU64,
    pub video_codec: std::sync::RwLock<Option<RTCRtpCodecCapability>>,
    pub audio_codec: std::sync::RwLock<Option<RTCRtpCodecCapability>>,

    // Screen share (additional video track)
    pub screen_tx: broadcast::Sender<webrtc::rtp::packet::Packet>,
    pub screen_ssrc: AtomicU64,
    pub screen_codec: std::sync::RwLock<Option<RTCRtpCodecCapability>>,

    /// What this publisher is sending (camera, screen, or both).
    pub track_source: std::sync::RwLock<TrackSource>,
}

impl Publisher {
    /// Create a new `Publisher` bound to the given peer connection.
    ///
    /// The broadcast channels are created with capacities of 300 (video) and
    /// 100 (audio) packets -- enough to absorb short subscriber stalls without
    /// blocking the publisher.
    pub fn new(peer_id: String, pc: Arc<RTCPeerConnection>) -> Self {
        let (video_tx, _) = broadcast::channel(300);
        let (audio_tx, _) = broadcast::channel(100);
        let (screen_tx, _) = broadcast::channel(300);
        Publisher {
            peer_id,
            pc,
            video_tx,
            audio_tx,
            video_ssrc: AtomicU64::new(0),
            video_codec: std::sync::RwLock::new(None),
            audio_codec: std::sync::RwLock::new(None),
            screen_tx,
            screen_ssrc: AtomicU64::new(0),
            screen_codec: std::sync::RwLock::new(None),
            track_source: std::sync::RwLock::new(TrackSource::Camera),
        }
    }

    /// Create a publisher specifically for screen sharing.
    pub fn new_screen(peer_id: String, pc: Arc<RTCPeerConnection>) -> Self {
        let p = Self::new(peer_id, pc);
        *p.track_source.write().unwrap() = TrackSource::Screen;
        p
    }

    /// Returns true if this publisher has an active screen share.
    pub fn has_screen(&self) -> bool {
        self.screen_ssrc.load(Ordering::Relaxed) != 0
    }
}

// ---------------------------------------------------------------------------
// Room
// ---------------------------------------------------------------------------

/// A room groups one or more publishers together with their subscribers.
pub struct Room {
    pub room_id: String,
    pub room_type: RoomType,
    pub max_publishers: usize,
    pub publishers: std::sync::RwLock<HashMap<String, Arc<Publisher>>>,
    pub subscriber_count: AtomicU64,
    pub created_at: std::time::Instant,
}

impl Room {
    /// Create an empty room.
    ///
    /// `max_publishers` is derived automatically from `room_type`:
    /// * `Broadcast`  -> 1  (+ screen shares don't count against this limit)
    /// * `Call`       -> 2
    /// * `Conference` -> 50 (configurable, but 50 is a sensible default)
    pub fn new(room_id: String, room_type: RoomType) -> Self {
        let max_publishers = match room_type {
            RoomType::Broadcast => 1,
            RoomType::Call => 2,
            RoomType::Conference => 50,
        };
        Room {
            room_id,
            room_type,
            max_publishers,
            publishers: std::sync::RwLock::new(HashMap::new()),
            subscriber_count: AtomicU64::new(0),
            created_at: std::time::Instant::now(),
        }
    }

    /// Create a room with a custom publisher limit (for Conference rooms).
    pub fn with_max_publishers(room_id: String, room_type: RoomType, max_publishers: usize) -> Self {
        let mut room = Self::new(room_id, room_type);
        room.max_publishers = max_publishers;
        room
    }

    /// Returns `true` when the room still has capacity for another publisher.
    pub fn can_publish(&self) -> bool {
        let pubs = self.publishers.read().unwrap();
        pubs.len() < self.max_publishers
    }

    /// Insert a publisher into the room.
    ///
    /// Fails with `"room is full"` when the publisher limit has already been
    /// reached.
    pub fn add_publisher(&self, publisher: Arc<Publisher>) -> Result<(), &'static str> {
        let mut pubs = self.publishers.write().unwrap();
        if pubs.len() >= self.max_publishers {
            return Err("room is full");
        }
        pubs.insert(publisher.peer_id.clone(), publisher);
        Ok(())
    }

    /// Remove a publisher by its peer id (no-op if absent).
    pub fn remove_publisher(&self, peer_id: &str) {
        let mut pubs = self.publishers.write().unwrap();
        pubs.remove(peer_id);
    }

    /// Snapshot of every publisher currently in the room.
    pub fn get_publishers(&self) -> Vec<Arc<Publisher>> {
        let pubs = self.publishers.read().unwrap();
        pubs.values().cloned().collect()
    }

    /// Get all publishers except the one with `exclude_peer_id`.
    /// Useful for conference mode: subscribe to everyone but yourself.
    pub fn get_other_publishers(&self, exclude_peer_id: &str) -> Vec<Arc<Publisher>> {
        let pubs = self.publishers.read().unwrap();
        pubs.values()
            .filter(|p| p.peer_id != exclude_peer_id)
            .cloned()
            .collect()
    }

    /// Current number of publishers.
    pub fn publisher_count(&self) -> usize {
        let pubs = self.publishers.read().unwrap();
        pubs.len()
    }

    /// Current number of subscribers (relaxed load -- eventual consistency is
    /// fine for metrics).
    pub fn subscriber_count(&self) -> u64 {
        self.subscriber_count.load(Ordering::Relaxed)
    }

    /// Build a serialisable summary of this room for API responses.
    pub fn info(&self) -> RoomInfo {
        RoomInfo {
            room_id: self.room_id.clone(),
            room_type: self.room_type,
            publisher_count: self.publisher_count(),
            subscriber_count: self.subscriber_count(),
            created_at_secs: self.created_at.elapsed().as_secs(),
        }
    }
}

// ---------------------------------------------------------------------------
// RoomInfo  (serialisable snapshot for the REST / JSON API)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct RoomInfo {
    pub room_id: String,
    pub room_type: RoomType,
    pub publisher_count: usize,
    pub subscriber_count: u64,
    /// Seconds elapsed since the room was created.
    pub created_at_secs: u64,
}
