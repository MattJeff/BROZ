use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use bytes::{BufMut, BytesMut};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};
use webrtc::util::marshal::Marshal;

use crate::error::ApiError;

// ---------------------------------------------------------------------------
// Architecture Decision: Option A — Raw RTP dump
// ---------------------------------------------------------------------------
//
// For a zero-copy SFU that never decodes media, Option A (raw RTP dump) is
// the best fit:
//
//   Option A: Save raw RTP packets with timestamps to a binary file.
//     + Zero additional dependencies (no GStreamer/FFmpeg).
//     + Zero CPU overhead — just memcpy packets to disk.
//     + Preserves all codec information losslessly.
//     + Can be converted to WebM/MP4 offline with `rtp2webm` or FFmpeg.
//     - Requires a post-processing step for playback.
//
//   Option B: Write WebM/MKV in real-time.
//     - Requires an RTP depacketizer (VP8 frames from RTP).
//     - Adds a WebM muxer dependency (ebml/matroska).
//     - Significant CPU cost per room being recorded.
//     - Defeats the purpose of a zero-copy SFU.
//
//   Option C: Pipe to GStreamer/FFmpeg.
//     - External process dependency.
//     - Operational complexity (process lifecycle management).
//     - But: zero Rust code for muxing. Good for production at scale.
//
// We implement Option A as the primary path (zero-dep, zero-copy), and
// provide an optional Option C hook (spawn FFmpeg subprocess) for users
// who want direct WebM output.
//
// File format: LiveRelay RTP Dump (.lrr)
//
//   File header (16 bytes):
//     [0..4]   magic: b"LRR1"
//     [4..8]   version: u32 LE = 1
//     [8..16]  start_timestamp_us: u64 LE (microseconds since UNIX epoch)
//
//   Per-packet record:
//     [0..4]   relative_timestamp_us: u32 LE (microseconds since recording start)
//     [4..5]   track_kind: u8 (0 = video, 1 = audio, 2 = screen)
//     [5..7]   packet_len: u16 LE
//     [7..7+N] raw RTP packet bytes
//

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const LRR_MAGIC: &[u8; 4] = b"LRR1";
const LRR_VERSION: u32 = 1;
const TRACK_VIDEO: u8 = 0;
const TRACK_AUDIO: u8 = 1;
const TRACK_SCREEN: u8 = 2;

// ---------------------------------------------------------------------------
// RecordingConfig
// ---------------------------------------------------------------------------

/// Configuration for the recording subsystem.
#[derive(Debug, Clone)]
pub struct RecordingConfig {
    /// Base directory for recording files.
    /// Each recording is stored as `{base_dir}/{room_id}/{recording_id}.lrr`
    pub base_dir: PathBuf,
    /// Maximum recording duration in seconds (0 = unlimited).
    pub max_duration_secs: u64,
}

impl Default for RecordingConfig {
    fn default() -> Self {
        Self {
            base_dir: PathBuf::from("./recordings"),
            max_duration_secs: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// RecordingHandle — the live state of an active recording
// ---------------------------------------------------------------------------

/// Tracks the state of a running recording session.
pub struct RecordingHandle {
    pub recording_id: String,
    pub room_id: String,
    pub file_path: PathBuf,
    pub started_at: std::time::Instant,
    pub started_at_unix: u64,
    pub cancel: CancellationToken,
    pub is_active: Arc<AtomicBool>,
    /// Optional: PID of external FFmpeg process for Option C.
    pub ffmpeg_pid: Option<u32>,
}

// ---------------------------------------------------------------------------
// RecordingInfo — serialisable snapshot for API responses
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct RecordingInfo {
    pub recording_id: String,
    pub room_id: String,
    pub file_path: String,
    pub duration_secs: u64,
    pub is_active: bool,
    pub started_at_unix: u64,
}

// ---------------------------------------------------------------------------
// RecordingManager — owns all active recordings
// ---------------------------------------------------------------------------

pub struct RecordingManager {
    pub config: RecordingConfig,
    pub active: std::sync::RwLock<HashMap<String, Arc<RecordingHandle>>>,
}

impl RecordingManager {
    pub fn new(config: RecordingConfig) -> Self {
        Self {
            config,
            active: std::sync::RwLock::new(HashMap::new()),
        }
    }

    /// Start recording a room. Subscribes to all publishers' broadcast
    /// channels and writes RTP packets to an .lrr file.
    pub async fn start_recording(
        self: &Arc<Self>,
        room: &Arc<crate::room::Room>,
    ) -> Result<RecordingInfo, ApiError> {
        let room_id = &room.room_id;

        // Check if already recording this room.
        {
            let active = self.active.read().unwrap();
            if active.values().any(|h| h.room_id == *room_id && h.is_active.load(Ordering::Relaxed))
            {
                return Err(ApiError::conflict(format!(
                    "Room '{room_id}' is already being recorded."
                )));
            }
        }

        // Create output directory.
        let room_dir = self.config.base_dir.join(room_id);
        tokio::fs::create_dir_all(&room_dir).await.map_err(|e| {
            warn!("Failed to create recording directory: {e}");
            ApiError::internal("Failed to create recording directory")
        })?;

        let recording_id = uuid::Uuid::new_v4().to_string();
        let file_path = room_dir.join(format!("{recording_id}.lrr"));

        let now_unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_micros() as u64;

        // Open file and write header.
        let mut file = File::create(&file_path).await.map_err(|e| {
            warn!("Failed to create recording file: {e}");
            ApiError::internal("Failed to create recording file")
        })?;

        let mut header = BytesMut::with_capacity(16);
        header.put_slice(LRR_MAGIC);
        header.put_u32_le(LRR_VERSION);
        header.put_u64_le(now_unix);

        file.write_all(&header).await.map_err(|e| {
            warn!("Failed to write recording header: {e}");
            ApiError::internal("Failed to write recording header")
        })?;

        let cancel = CancellationToken::new();
        let is_active = Arc::new(AtomicBool::new(true));

        let handle = Arc::new(RecordingHandle {
            recording_id: recording_id.clone(),
            room_id: room_id.clone(),
            file_path: file_path.clone(),
            started_at: std::time::Instant::now(),
            started_at_unix: now_unix / 1_000_000, // seconds
            cancel: cancel.clone(),
            is_active: is_active.clone(),
            ffmpeg_pid: None,
        });

        // Insert into active recordings.
        {
            let mut active = self.active.write().unwrap();
            active.insert(recording_id.clone(), handle.clone());
        }

        // Spawn the recording writer task.
        let publishers = room.get_publishers();
        let start_us = now_unix;
        let max_dur = self.config.max_duration_secs;
        let _manager = Arc::clone(self);
        let rec_id = recording_id.clone();

        tokio::spawn(async move {
            let result = recording_writer_task(
                file,
                publishers,
                cancel,
                is_active.clone(),
                start_us,
                max_dur,
            )
            .await;

            is_active.store(false, Ordering::Relaxed);

            if let Err(e) = result {
                warn!("Recording '{rec_id}' writer error: {e}");
            } else {
                info!("Recording '{rec_id}' completed");
            }
        });

        let info = RecordingInfo {
            recording_id,
            room_id: room_id.clone(),
            file_path: file_path.to_string_lossy().to_string(),
            duration_secs: 0,
            is_active: true,
            started_at_unix: now_unix / 1_000_000,
        };

        info!("Recording started for room '{room_id}': {}", info.file_path);
        Ok(info)
    }

    /// Stop an active recording by recording_id.
    pub fn stop_recording(&self, recording_id: &str) -> Result<RecordingInfo, ApiError> {
        let active = self.active.read().unwrap();
        let handle = active
            .get(recording_id)
            .ok_or_else(|| ApiError::not_found(format!("Recording '{recording_id}' not found.")))?;

        if !handle.is_active.load(Ordering::Relaxed) {
            return Err(ApiError::conflict(format!(
                "Recording '{recording_id}' is already stopped."
            )));
        }

        handle.cancel.cancel();
        handle.is_active.store(false, Ordering::Relaxed);

        let duration = handle.started_at.elapsed().as_secs();

        let info = RecordingInfo {
            recording_id: handle.recording_id.clone(),
            room_id: handle.room_id.clone(),
            file_path: handle.file_path.to_string_lossy().to_string(),
            duration_secs: duration,
            is_active: false,
            started_at_unix: handle.started_at_unix,
        };

        info!(
            "Recording '{}' stopped for room '{}' ({}s)",
            handle.recording_id, handle.room_id, duration
        );

        Ok(info)
    }

    /// Stop all recordings for a given room.
    pub fn stop_room_recordings(&self, room_id: &str) -> Vec<RecordingInfo> {
        let active = self.active.read().unwrap();
        let mut stopped = Vec::new();

        for handle in active.values() {
            if handle.room_id == room_id && handle.is_active.load(Ordering::Relaxed) {
                handle.cancel.cancel();
                handle.is_active.store(false, Ordering::Relaxed);

                stopped.push(RecordingInfo {
                    recording_id: handle.recording_id.clone(),
                    room_id: handle.room_id.clone(),
                    file_path: handle.file_path.to_string_lossy().to_string(),
                    duration_secs: handle.started_at.elapsed().as_secs(),
                    is_active: false,
                    started_at_unix: handle.started_at_unix,
                });
            }
        }

        stopped
    }

    /// List all recordings (active and completed).
    pub fn list_recordings(&self, room_id: Option<&str>) -> Vec<RecordingInfo> {
        let active = self.active.read().unwrap();
        active
            .values()
            .filter(|h| room_id.map_or(true, |rid| h.room_id == rid))
            .map(|h| RecordingInfo {
                recording_id: h.recording_id.clone(),
                room_id: h.room_id.clone(),
                file_path: h.file_path.to_string_lossy().to_string(),
                duration_secs: h.started_at.elapsed().as_secs(),
                is_active: h.is_active.load(Ordering::Relaxed),
                started_at_unix: h.started_at_unix,
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Recording writer task — the core I/O loop
// ---------------------------------------------------------------------------

/// Internal task that reads from broadcast channels and writes .lrr packets.
async fn recording_writer_task(
    mut file: File,
    publishers: Vec<Arc<crate::room::Publisher>>,
    cancel: CancellationToken,
    is_active: Arc<AtomicBool>,
    start_us: u64,
    max_duration_secs: u64,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Subscribe to all publishers' broadcast channels.
    // We use a select! over all receivers simultaneously.
    // For simplicity with dynamic publisher count, we merge into a single
    // mpsc channel.

    let (merge_tx, mut merge_rx) =
        tokio::sync::mpsc::channel::<(u8, webrtc::rtp::packet::Packet)>(512);

    for publisher in &publishers {
        // Video
        {
            let mut rx = publisher.video_tx.subscribe();
            let tx = merge_tx.clone();
            let cancel = cancel.clone();
            let track_kind = if publisher.peer_id.ends_with("-screen") {
                TRACK_SCREEN
            } else {
                TRACK_VIDEO
            };

            tokio::spawn(async move {
                loop {
                    tokio::select! {
                        _ = cancel.cancelled() => break,
                        result = rx.recv() => {
                            match result {
                                Ok(pkt) => {
                                    if tx.send((track_kind, pkt)).await.is_err() {
                                        break;
                                    }
                                }
                                Err(broadcast::error::RecvError::Lagged(n)) => {
                                    warn!("Recording subscriber lagged, skipped {n} video packets");
                                }
                                Err(broadcast::error::RecvError::Closed) => break,
                            }
                        }
                    }
                }
            });
        }

        // Audio
        {
            let mut rx = publisher.audio_tx.subscribe();
            let tx = merge_tx.clone();
            let cancel = cancel.clone();

            tokio::spawn(async move {
                loop {
                    tokio::select! {
                        _ = cancel.cancelled() => break,
                        result = rx.recv() => {
                            match result {
                                Ok(pkt) => {
                                    if tx.send((TRACK_AUDIO, pkt)).await.is_err() {
                                        break;
                                    }
                                }
                                Err(broadcast::error::RecvError::Lagged(n)) => {
                                    warn!("Recording subscriber lagged, skipped {n} audio packets");
                                }
                                Err(broadcast::error::RecvError::Closed) => break,
                            }
                        }
                    }
                }
            });
        }
    }

    // Drop the original sender so merge_rx completes when all spawned tasks end.
    drop(merge_tx);

    let max_deadline = if max_duration_secs > 0 {
        Some(tokio::time::Instant::now() + std::time::Duration::from_secs(max_duration_secs))
    } else {
        None
    };

    // Write buffer: batch small packets for fewer syscalls.
    let mut write_buf = BytesMut::with_capacity(65536);

    loop {
        // Check max duration.
        if let Some(deadline) = max_deadline {
            if tokio::time::Instant::now() >= deadline {
                info!("Recording reached max duration ({max_duration_secs}s), stopping");
                break;
            }
        }

        tokio::select! {
            _ = cancel.cancelled() => break,
            maybe_pkt = merge_rx.recv() => {
                match maybe_pkt {
                    Some((track_kind, pkt)) => {
                        // Serialize RTP packet to bytes.
                        let rtp_bytes = match pkt.marshal() {
                            Ok(b) => b,
                            Err(e) => {
                                warn!("RTP marshal error during recording: {e}");
                                continue;
                            }
                        };

                        let now_us = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_micros() as u64;
                        let relative_us = (now_us.saturating_sub(start_us)) as u32;

                        let pkt_len = rtp_bytes.len() as u16;

                        // Write record: 4 + 1 + 2 + N bytes.
                        write_buf.put_u32_le(relative_us);
                        write_buf.put_u8(track_kind);
                        write_buf.put_u16_le(pkt_len);
                        write_buf.put_slice(&rtp_bytes);

                        // Flush if buffer is getting large.
                        if write_buf.len() >= 32768 {
                            file.write_all(&write_buf).await?;
                            write_buf.clear();
                        }
                    }
                    None => {
                        // All publishers dropped.
                        info!("All publisher channels closed, stopping recording");
                        break;
                    }
                }
            }
        }
    }

    // Flush remaining data.
    if !write_buf.is_empty() {
        file.write_all(&write_buf).await?;
    }
    file.flush().await?;

    is_active.store(false, Ordering::Relaxed);
    Ok(())
}

// ---------------------------------------------------------------------------
// Option C: FFmpeg subprocess recording (optional)
// ---------------------------------------------------------------------------

/// Start an FFmpeg process that reads raw RTP from a UDP socket.
/// The SFU sends a copy of the RTP packets to localhost:{port}.
/// FFmpeg muxes them into a WebM/MKV file.
///
/// Usage:
///   let (pid, port) = start_ffmpeg_recording("/path/to/output.webm")?;
///   // Then send RTP packets to 127.0.0.1:port
///
/// This is provided as an alternative for deployments that want direct
/// WebM output without post-processing.
#[allow(dead_code)]
pub fn start_ffmpeg_recording(
    output_path: &str,
) -> Result<(u32, u16), Box<dyn std::error::Error>> {
    use std::process::Command;

    // Pick a random port in the ephemeral range.
    let port: u16 = 20000 + (rand::random::<u16>() % 10000);

    // Build the FFmpeg command:
    //   ffmpeg -protocol_whitelist file,rtp,udp \
    //          -i rtp://127.0.0.1:{port} \
    //          -c:v copy -c:a copy \
    //          -f webm {output_path}
    //
    // NOTE: In practice you'd pass an SDP file describing the codecs.
    // This is a simplified example.
    let child = Command::new("ffmpeg")
        .args([
            "-y",
            "-protocol_whitelist",
            "file,rtp,udp",
            "-f",
            "rtp",
            "-i",
            &format!("rtp://127.0.0.1:{port}"),
            "-c:v",
            "copy",
            "-c:a",
            "copy",
            "-f",
            "webm",
            output_path,
        ])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    let pid = child.id();
    info!("FFmpeg recording started: pid={pid}, port={port}, output={output_path}");

    Ok((pid, port))
}

// ---------------------------------------------------------------------------
// API Handlers
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct StartRecordingRequest {
    /// Optional: "lrr" (default) or "ffmpeg"
    #[serde(default = "default_format")]
    #[allow(dead_code)]
    pub format: String,
}

fn default_format() -> String {
    "lrr".to_string()
}

/// POST /v1/rooms/:room_id/recording/start
pub async fn start_recording(
    State(state): State<Arc<crate::AppState>>,
    Path(room_id): Path<String>,
    headers: HeaderMap,
    Json(_body): Json<StartRecordingRequest>,
) -> Result<Json<RecordingInfo>, ApiError> {
    // Require API key.
    crate::auth::require_api_key(&headers, &state.api_keys).await?;

    // Look up the room.
    let room = {
        let rooms = state.rooms.read().unwrap();
        rooms.get(&room_id).cloned()
    };
    let room = room.ok_or_else(|| ApiError::room_not_found(&room_id))?;

    // Check there are publishers to record.
    if room.publisher_count() == 0 {
        return Err(ApiError::bad_request(format!(
            "Room '{room_id}' has no publishers to record."
        )));
    }

    let recording_mgr = state
        .recording
        .as_ref()
        .ok_or_else(|| ApiError::internal("Recording subsystem not initialized"))?;

    let info = recording_mgr.start_recording(&room).await?;

    Ok(Json(info))
}

/// POST /v1/rooms/:room_id/recording/stop
pub async fn stop_recording(
    State(state): State<Arc<crate::AppState>>,
    Path(room_id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<Vec<RecordingInfo>>, ApiError> {
    crate::auth::require_api_key(&headers, &state.api_keys).await?;

    let recording_mgr = state
        .recording
        .as_ref()
        .ok_or_else(|| ApiError::internal("Recording subsystem not initialized"))?;

    let stopped = recording_mgr.stop_room_recordings(&room_id);

    if stopped.is_empty() {
        return Err(ApiError::not_found(format!(
            "No active recording found for room '{room_id}'."
        )));
    }

    Ok(Json(stopped))
}

/// GET /v1/rooms/:room_id/recordings
pub async fn list_room_recordings(
    State(state): State<Arc<crate::AppState>>,
    Path(room_id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<Vec<RecordingInfo>>, ApiError> {
    crate::auth::require_api_key(&headers, &state.api_keys).await?;

    let recording_mgr = state
        .recording
        .as_ref()
        .ok_or_else(|| ApiError::internal("Recording subsystem not initialized"))?;

    let recordings = recording_mgr.list_recordings(Some(&room_id));
    Ok(Json(recordings))
}

// ---------------------------------------------------------------------------
// LRR file reader (for playback/conversion tools)
// ---------------------------------------------------------------------------

/// A single record from an .lrr file.
#[derive(Debug)]
#[allow(dead_code)]
pub struct LrrRecord {
    pub relative_timestamp_us: u32,
    pub track_kind: u8,
    pub rtp_data: Vec<u8>,
}

/// Read and parse an .lrr file header. Returns (version, start_timestamp_us).
#[allow(dead_code)]
pub fn read_lrr_header(data: &[u8]) -> Result<(u32, u64), &'static str> {
    if data.len() < 16 {
        return Err("File too small for LRR header");
    }
    if &data[0..4] != LRR_MAGIC {
        return Err("Invalid LRR magic bytes");
    }
    let version = u32::from_le_bytes(data[4..8].try_into().unwrap());
    let start_ts = u64::from_le_bytes(data[8..16].try_into().unwrap());
    Ok((version, start_ts))
}

/// Parse all records from an .lrr file (after the 16-byte header).
#[allow(dead_code)]
pub fn read_lrr_records(data: &[u8]) -> Vec<LrrRecord> {
    let mut records = Vec::new();
    let mut offset = 16; // skip header

    while offset + 7 <= data.len() {
        let relative_ts = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap());
        let track_kind = data[offset + 4];
        let pkt_len = u16::from_le_bytes(data[offset + 5..offset + 7].try_into().unwrap()) as usize;

        if offset + 7 + pkt_len > data.len() {
            break;
        }

        let rtp_data = data[offset + 7..offset + 7 + pkt_len].to_vec();
        records.push(LrrRecord {
            relative_timestamp_us: relative_ts,
            track_kind,
            rtp_data,
        });

        offset += 7 + pkt_len;
    }

    records
}
