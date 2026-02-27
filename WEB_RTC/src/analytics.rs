// src/analytics.rs
//
// WebRTC quality analytics for LiveRelay.
//
// ─ Architecture ─────────────────────────────────────────────────────────────
//
//   ┌─────────────────────┐
//   │   StatsCollector     │  (background task, runs every N seconds)
//   │                      │
//   │  for each room:      │
//   │    for each peer:    │
//   │      get_stats()     │──> computes QualityMetrics
//   │      check thresholds│──> emits quality.degraded via EventBus
//   │      store snapshot  │──> available via GET /v1/analytics
//   └─────────────────────┘
//
// ─ Metrics ──────────────────────────────────────────────────────────────────
//
//   - round_trip_time_ms  : ICE candidate-pair RTT
//   - packet_loss_pct     : lost / (lost + received) * 100
//   - bitrate_kbps        : bytes_sent delta / interval
//   - jitter_ms           : inter-arrival jitter (from RTP)
//   - mos_score           : Mean Opinion Score estimate (1.0 - 5.0)
//
// ─ MOS estimation ───────────────────────────────────────────────────────────
//
//   We use the E-model simplified formula (ITU-T G.107):
//     R = 93.2 - Id - Ie
//     Id = 0.024 * d + 0.11 * (d - 177.3) * H(d - 177.3)
//     Ie = codec_ie + (95 - codec_ie) * ppl / (ppl + bpl)
//     MOS = 1 + 0.035*R + R*(R-60)*(100-R)*7e-6
//
//   For VP8/Opus defaults: codec_ie = 0, bpl = 25.1.
//
// ────────────────────────────────────────────────────────────────────────────

use axum::extract::State;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, info};

use crate::events::{EventBus, LiveRelayEvent};

// ─── Quality metrics ────────────────────────────────────────────────────────

/// A snapshot of quality metrics for a single peer connection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityMetrics {
    pub room_id: String,
    pub peer_id: String,

    /// Round-trip time in milliseconds (from ICE candidate pair stats).
    pub round_trip_time_ms: f64,

    /// Packet loss percentage (0.0 - 100.0).
    pub packet_loss_pct: f64,

    /// Current sending or receiving bitrate in kbps.
    pub bitrate_kbps: f64,

    /// Inter-arrival jitter in milliseconds.
    pub jitter_ms: f64,

    /// Estimated Mean Opinion Score (1.0 - 5.0).
    pub mos_score: f64,

    /// Unix timestamp of this measurement.
    pub timestamp: u64,
}

// ─── MOS estimation ─────────────────────────────────────────────────────────

/// Estimate the MOS score from delay (ms) and packet loss percentage.
///
/// Based on the ITU-T G.107 E-model simplified for VoIP.
pub fn estimate_mos(delay_ms: f64, packet_loss_pct: f64) -> f64 {
    // Delay impairment (Id).
    let d = delay_ms;
    let h = if d > 177.3 { 1.0 } else { 0.0 };
    let id = 0.024 * d + 0.11 * (d - 177.3) * h;

    // Equipment impairment (Ie) for wideband codec (Opus).
    // codec_ie = 0 for Opus, bpl = 25.1 (packet loss robustness).
    let codec_ie = 0.0_f64;
    let bpl = 25.1_f64;
    let ppl = packet_loss_pct;
    let ie = codec_ie + (95.0 - codec_ie) * ppl / (ppl + bpl);

    // R factor.
    let r = (93.2 - id - ie).clamp(0.0, 100.0);

    // Convert R to MOS.
    if r < 6.5 {
        1.0
    } else {
        let mos = 1.0 + 0.035 * r + r * (r - 60.0) * (100.0 - r) * 7.0e-6;
        mos.clamp(1.0, 5.0)
    }
}

// ─── Degradation thresholds ─────────────────────────────────────────────────

/// Configurable thresholds that trigger `quality.degraded` events.
#[derive(Debug, Clone)]
pub struct QualityThresholds {
    /// Emit event when RTT exceeds this value (ms).
    pub max_rtt_ms: f64,
    /// Emit event when packet loss exceeds this percentage.
    pub max_packet_loss_pct: f64,
    /// Emit event when MOS drops below this score.
    pub min_mos: f64,
    /// Emit event when jitter exceeds this value (ms).
    pub max_jitter_ms: f64,
}

impl Default for QualityThresholds {
    fn default() -> Self {
        Self {
            max_rtt_ms: 300.0,
            max_packet_loss_pct: 5.0,
            min_mos: 3.0,
            max_jitter_ms: 50.0,
        }
    }
}

// ─── Stats store ────────────────────────────────────────────────────────────

/// In-memory store of the latest quality metrics per peer.
///
/// Key: `(room_id, peer_id)`.
#[derive(Clone, Default)]
pub struct AnalyticsStore {
    inner: Arc<RwLock<HashMap<(String, String), QualityMetrics>>>,
}

impl AnalyticsStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert or update metrics for a peer.
    pub async fn upsert(&self, metrics: QualityMetrics) {
        let key = (metrics.room_id.clone(), metrics.peer_id.clone());
        let mut map = self.inner.write().await;
        map.insert(key, metrics);
    }

    /// Remove metrics for a peer (e.g. on disconnect).
    pub async fn remove(&self, room_id: &str, peer_id: &str) {
        let mut map = self.inner.write().await;
        map.remove(&(room_id.to_string(), peer_id.to_string()));
    }

    /// Get all metrics, optionally filtered by room.
    pub async fn list(&self, room_id: Option<&str>) -> Vec<QualityMetrics> {
        let map = self.inner.read().await;
        map.values()
            .filter(|m| room_id.map_or(true, |r| m.room_id == r))
            .cloned()
            .collect()
    }

    /// Get metrics for a specific peer.
    pub async fn get(&self, room_id: &str, peer_id: &str) -> Option<QualityMetrics> {
        let map = self.inner.read().await;
        map.get(&(room_id.to_string(), peer_id.to_string())).cloned()
    }
}

// ─── Stats collection from webrtc-rs ────────────────────────────────────────

/// Raw counters extracted from a `RTCPeerConnection::get_stats()` call.
///
/// In webrtc-rs, `get_stats()` returns a `StatsReport` containing
/// `ICECandidatePairStats`, `InboundRTPStreamStats`, `OutboundRTPStreamStats`,
/// etc.  This struct normalises the fields we care about.
#[derive(Debug, Clone, Default)]
pub struct RawPeerStats {
    pub room_id: String,
    pub peer_id: String,

    /// Total bytes sent since start.
    pub bytes_sent: u64,
    /// Total bytes received since start.
    pub bytes_received: u64,
    /// Total packets lost (cumulative).
    pub packets_lost: u64,
    /// Total packets received (cumulative).
    pub packets_received: u64,
    /// Current round-trip time (seconds, from ICE candidate pair).
    pub current_rtt_secs: f64,
    /// Jitter in seconds (from RTP receiver stats).
    pub jitter_secs: f64,
}

/// Compute `QualityMetrics` from the current and previous raw stats snapshots.
///
/// The delta between two snapshots gives us per-interval rates (bitrate,
/// packet loss rate) rather than cumulative values.
pub fn compute_metrics(
    current: &RawPeerStats,
    previous: Option<&RawPeerStats>,
    interval: Duration,
) -> QualityMetrics {
    let interval_secs = interval.as_secs_f64();

    // Bitrate.
    let bytes_delta = match previous {
        Some(prev) => current.bytes_sent.saturating_sub(prev.bytes_sent)
            + current.bytes_received.saturating_sub(prev.bytes_received),
        None => current.bytes_sent + current.bytes_received,
    };
    let bitrate_kbps = (bytes_delta as f64 * 8.0) / (interval_secs * 1000.0);

    // Packet loss.
    let (lost_delta, received_delta) = match previous {
        Some(prev) => (
            current.packets_lost.saturating_sub(prev.packets_lost),
            current.packets_received.saturating_sub(prev.packets_received),
        ),
        None => (current.packets_lost, current.packets_received),
    };
    let total = lost_delta + received_delta;
    let packet_loss_pct = if total > 0 {
        (lost_delta as f64 / total as f64) * 100.0
    } else {
        0.0
    };

    let rtt_ms = current.current_rtt_secs * 1000.0;
    let jitter_ms = current.jitter_secs * 1000.0;
    let mos = estimate_mos(rtt_ms, packet_loss_pct);

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    QualityMetrics {
        room_id: current.room_id.clone(),
        peer_id: current.peer_id.clone(),
        round_trip_time_ms: rtt_ms,
        packet_loss_pct,
        bitrate_kbps,
        jitter_ms,
        mos_score: mos,
        timestamp: now,
    }
}

// ─── Threshold checking ─────────────────────────────────────────────────────

/// Check metrics against thresholds and emit `quality.degraded` events.
pub fn check_thresholds(
    metrics: &QualityMetrics,
    thresholds: &QualityThresholds,
    bus: &EventBus,
) {
    if metrics.round_trip_time_ms > thresholds.max_rtt_ms {
        bus.emit(LiveRelayEvent::quality_degraded(
            &metrics.room_id,
            &metrics.peer_id,
            "round_trip_time_ms",
            metrics.round_trip_time_ms,
            thresholds.max_rtt_ms,
            "above",
        ));
    }

    if metrics.packet_loss_pct > thresholds.max_packet_loss_pct {
        bus.emit(LiveRelayEvent::quality_degraded(
            &metrics.room_id,
            &metrics.peer_id,
            "packet_loss_pct",
            metrics.packet_loss_pct,
            thresholds.max_packet_loss_pct,
            "above",
        ));
    }

    if metrics.jitter_ms > thresholds.max_jitter_ms {
        bus.emit(LiveRelayEvent::quality_degraded(
            &metrics.room_id,
            &metrics.peer_id,
            "jitter_ms",
            metrics.jitter_ms,
            thresholds.max_jitter_ms,
            "above",
        ));
    }

    if metrics.mos_score < thresholds.min_mos {
        bus.emit(LiveRelayEvent::quality_degraded(
            &metrics.room_id,
            &metrics.peer_id,
            "mos_score",
            metrics.mos_score,
            thresholds.min_mos,
            "below",
        ));
    }
}

// ─── Background stats collector ─────────────────────────────────────────────

/// Spawn the periodic stats collection task.
///
/// Every `interval`, this task iterates over all rooms and publishers, calls
/// `get_stats()` on their peer connections, computes quality metrics, stores
/// them, and checks degradation thresholds.
///
/// **Note**: `webrtc-rs` exposes `RTCPeerConnection::get_stats()` which
/// returns a `StatsReport`.  The actual extraction logic depends on the
/// webrtc-rs version.  Below is the integration pattern -- the extraction
/// function `extract_raw_stats` should be adapted to the concrete
/// `StatsReport` layout.
pub fn spawn_stats_collector(
    state: Arc<crate::AppState>,
    interval: Duration,
    thresholds: QualityThresholds,
) -> tokio::task::JoinHandle<()> {
    let bus = state.event_bus.clone();
    let store = state.analytics.clone();

    // Previous stats for delta computation.
    let prev_stats: Arc<RwLock<HashMap<(String, String), RawPeerStats>>> =
        Arc::new(RwLock::new(HashMap::new()));

    tokio::spawn(async move {
        info!(
            interval_ms = interval.as_millis() as u64,
            "analytics stats collector started"
        );

        let mut ticker = tokio::time::interval(interval);

        loop {
            ticker.tick().await;

            // Snapshot current rooms.
            let rooms: Vec<(String, Vec<(String, Arc<webrtc::peer_connection::RTCPeerConnection>)>)> = {
                let rooms_map = state.rooms.read().unwrap();
                rooms_map
                    .iter()
                    .map(|(rid, room)| {
                        let publishers = room.get_publishers();
                        let peers: Vec<_> = publishers
                            .iter()
                            .map(|p| (p.peer_id.clone(), p.pc.clone()))
                            .collect();
                        (rid.clone(), peers)
                    })
                    .collect()
            };

            for (room_id, peers) in &rooms {
                for (peer_id, pc) in peers {
                    // Get stats from the peer connection.
                    let stats_report = pc.get_stats().await;

                    let raw = extract_raw_stats(room_id, peer_id, &stats_report);

                    let prev = {
                        let map = prev_stats.read().await;
                        map.get(&(room_id.clone(), peer_id.clone())).cloned()
                    };

                    let metrics = compute_metrics(&raw, prev.as_ref(), interval);

                    debug!(
                        room_id = %room_id,
                        peer_id = %peer_id,
                        rtt_ms = metrics.round_trip_time_ms,
                        loss_pct = metrics.packet_loss_pct,
                        bitrate = metrics.bitrate_kbps,
                        mos = metrics.mos_score,
                        "stats collected"
                    );

                    // Store.
                    store.upsert(metrics.clone()).await;

                    // Check thresholds.
                    check_thresholds(&metrics, &thresholds, &bus);

                    // Remember for next delta.
                    {
                        let mut map = prev_stats.write().await;
                        map.insert((room_id.clone(), peer_id.clone()), raw);
                    }
                }
            }
        }
    })
}

/// Extract `RawPeerStats` from a webrtc-rs `StatsReport`.
///
/// The `StatsReport` is a `HashMap<String, StatsReportType>`.  We iterate
/// through the entries looking for candidate-pair and RTP stream stats.
fn extract_raw_stats(
    room_id: &str,
    peer_id: &str,
    report: &webrtc::stats::StatsReport,
) -> RawPeerStats {
    let mut raw = RawPeerStats {
        room_id: room_id.to_string(),
        peer_id: peer_id.to_string(),
        ..Default::default()
    };

    for (_key, stat) in &report.reports {
        match stat {
            webrtc::stats::StatsReportType::CandidatePair(cp) => {
                raw.current_rtt_secs = cp.current_round_trip_time;
                raw.bytes_sent += cp.bytes_sent as u64;
                raw.bytes_received += cp.bytes_received as u64;
            }
            webrtc::stats::StatsReportType::InboundRTP(inbound) => {
                raw.packets_received += inbound.packets_received;
                // Note: webrtc-rs 0.11 doesn't expose packets_lost or jitter
                // on InboundRTPStats (marked as TODO upstream).
                // We rely on candidate-pair RTT for quality estimation.
            }
            webrtc::stats::StatsReportType::OutboundRTP(outbound) => {
                raw.bytes_sent += outbound.bytes_sent as u64;
                raw.packets_received += outbound.packets_sent as u64;
            }
            _ => {}
        }
    }

    raw
}

// ─── Analytics API handler ──────────────────────────────────────────────────

/// Query parameters for `GET /v1/analytics`.
#[derive(Debug, Deserialize)]
pub struct AnalyticsQuery {
    pub room_id: Option<String>,
}

/// `GET /v1/analytics` -- retrieve current quality metrics.
pub async fn get_analytics(
    State(state): State<Arc<crate::AppState>>,
    headers: axum::http::HeaderMap,
    axum::extract::Query(query): axum::extract::Query<AnalyticsQuery>,
) -> Result<axum::Json<Vec<QualityMetrics>>, crate::error::ApiError> {
    crate::auth::require_api_key(&headers, &state.api_keys).await?;

    let metrics = state
        .analytics
        .list(query.room_id.as_deref())
        .await;

    Ok(axum::Json(metrics))
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mos_perfect_conditions() {
        // Zero delay, zero loss -> high MOS.
        let mos = estimate_mos(0.0, 0.0);
        assert!(mos > 4.3, "MOS should be excellent: {mos}");
    }

    #[test]
    fn mos_degraded_by_loss() {
        let good = estimate_mos(50.0, 0.0);
        let bad = estimate_mos(50.0, 10.0);
        assert!(good > bad, "Loss should reduce MOS: good={good}, bad={bad}");
    }

    #[test]
    fn mos_degraded_by_delay() {
        let good = estimate_mos(50.0, 0.0);
        let bad = estimate_mos(500.0, 0.0);
        assert!(good > bad, "Delay should reduce MOS: good={good}, bad={bad}");
    }

    #[test]
    fn mos_clamped() {
        let mos = estimate_mos(2000.0, 50.0);
        assert!(mos >= 1.0, "MOS minimum is 1.0: {mos}");
        assert!(mos <= 5.0, "MOS maximum is 5.0: {mos}");
    }

    #[test]
    fn compute_metrics_basic() {
        let current = RawPeerStats {
            room_id: "r1".into(),
            peer_id: "p1".into(),
            bytes_sent: 100_000,
            bytes_received: 200_000,
            packets_lost: 5,
            packets_received: 995,
            current_rtt_secs: 0.05,
            jitter_secs: 0.01,
        };

        let metrics = compute_metrics(&current, None, Duration::from_secs(5));

        assert_eq!(metrics.room_id, "r1");
        assert_eq!(metrics.peer_id, "p1");
        assert_eq!(metrics.round_trip_time_ms, 50.0);
        assert!(metrics.packet_loss_pct > 0.0 && metrics.packet_loss_pct < 1.0);
        assert!(metrics.bitrate_kbps > 0.0);
        assert_eq!(metrics.jitter_ms, 10.0);
        assert!(metrics.mos_score > 1.0);
    }

    #[test]
    fn compute_metrics_delta() {
        let prev = RawPeerStats {
            room_id: "r1".into(),
            peer_id: "p1".into(),
            bytes_sent: 50_000,
            bytes_received: 100_000,
            packets_lost: 2,
            packets_received: 500,
            current_rtt_secs: 0.04,
            jitter_secs: 0.008,
        };

        let current = RawPeerStats {
            room_id: "r1".into(),
            peer_id: "p1".into(),
            bytes_sent: 100_000,
            bytes_received: 200_000,
            packets_lost: 5,
            packets_received: 995,
            current_rtt_secs: 0.05,
            jitter_secs: 0.01,
        };

        let metrics = compute_metrics(&current, Some(&prev), Duration::from_secs(5));

        // Delta bytes: (100000-50000) + (200000-100000) = 150000
        // Bitrate: 150000 * 8 / (5 * 1000) = 240 kbps
        assert!((metrics.bitrate_kbps - 240.0).abs() < 0.1);

        // Delta lost: 3, delta received: 495. Total: 498.
        // Loss: 3/498 * 100 ~ 0.60%
        assert!(metrics.packet_loss_pct > 0.5 && metrics.packet_loss_pct < 0.7);
    }

    #[tokio::test]
    async fn analytics_store_crud() {
        let store = AnalyticsStore::new();

        let m = QualityMetrics {
            room_id: "r1".into(),
            peer_id: "p1".into(),
            round_trip_time_ms: 42.0,
            packet_loss_pct: 1.5,
            bitrate_kbps: 1500.0,
            jitter_ms: 5.0,
            mos_score: 4.2,
            timestamp: 0,
        };

        store.upsert(m.clone()).await;
        assert_eq!(store.list(None).await.len(), 1);
        assert_eq!(store.list(Some("r1")).await.len(), 1);
        assert_eq!(store.list(Some("r2")).await.len(), 0);

        store.remove("r1", "p1").await;
        assert_eq!(store.list(None).await.len(), 0);
    }
}
