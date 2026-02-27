use axum::{
    extract::State,
    http::HeaderMap,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};
use webrtc::api::interceptor_registry::register_default_interceptors;
use webrtc::api::media_engine::{MediaEngine, MIME_TYPE_OPUS, MIME_TYPE_VP8};
use webrtc::api::APIBuilder;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::interceptor::registry::Registry;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::rtcp::payload_feedbacks::picture_loss_indication::PictureLossIndication;
use webrtc::rtp_transceiver::rtp_codec::{RTCRtpCodecCapability, RTPCodecType};
use webrtc::track::track_local::track_local_static_rtp::TrackLocalStaticRTP;
use webrtc::track::track_local::{TrackLocal, TrackLocalWriter};

use crate::config::Config;
use crate::error::ApiError;
use crate::room::Publisher;

// ─── JWT extraction helper ───────────────────────────────────────────────────

fn extract_bearer_token(headers: &HeaderMap) -> Result<&str, ApiError> {
    headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(ApiError::auth_header_missing)
}

// ─── DTOs ────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct SdpOffer {
    pub sdp: String,
    #[serde(rename = "type")]
    #[allow(dead_code)]
    pub sdp_type: String,
    /// If true, this is a screen-share publish (uses getDisplayMedia on client).
    /// The server routes video RTP to the `screen_tx` broadcast channel
    /// instead of `video_tx`, so subscribers can distinguish camera vs screen.
    #[serde(default)]
    pub screen: bool,
}

#[derive(Serialize)]
pub struct SdpAnswer {
    pub sdp: String,
    #[serde(rename = "type")]
    pub sdp_type: String,
}

// ─── PeerConnection factory ─────────────────────────────────────────────────

/// Create a new `RTCPeerConnection` using the ICE servers from the
/// production configuration (STUN + TURN).
async fn create_peer_connection(cfg: &Config) -> Result<Arc<RTCPeerConnection>, webrtc::Error> {
    let mut media_engine = MediaEngine::default();
    media_engine.register_default_codecs()?;

    let mut registry = Registry::new();
    registry = register_default_interceptors(registry, &mut media_engine)?;

    // Configure UDP port range + NAT1To1 for Docker compatibility
    let mut setting_engine = webrtc::api::setting_engine::SettingEngine::default();
    if cfg.udp_port_min > 0 && cfg.udp_port_max > 0 {
        let ephemeral = webrtc::ice::udp_network::EphemeralUDP::new(cfg.udp_port_min, cfg.udp_port_max)
            .map_err(|e| webrtc::Error::new(format!("invalid UDP port range: {e}")))?;
        setting_engine.set_udp_network(webrtc::ice::udp_network::UDPNetwork::Ephemeral(ephemeral));
        info!("WebRTC UDP port range: {}-{}", cfg.udp_port_min, cfg.udp_port_max);
    }
    // Replace container-internal IPs with the public IP in ICE candidates
    // so browsers (on the host) can reach the SFU through Docker port mappings.
    // Resolve hostname to IP if needed (NAT1To1 requires IP addresses, not hostnames).
    let nat_ip = if cfg.public_host == "localhost" {
        "127.0.0.1".to_string()
    } else {
        cfg.public_host.clone()
    };
    setting_engine.set_nat_1to1_ips(
        vec![nat_ip],
        webrtc::ice_transport::ice_candidate_type::RTCIceCandidateType::Host,
    );

    let api = APIBuilder::new()
        .with_media_engine(media_engine)
        .with_interceptor_registry(registry)
        .with_setting_engine(setting_engine)
        .build();

    // Build ICE servers from configuration (STUN only for server-side).
    let ice_servers: Vec<RTCIceServer> = cfg
        .ice_servers_for_server()
        .into_iter()
        .map(|s| RTCIceServer {
            urls: s.urls,
            username: s.username.unwrap_or_default(),
            credential: s.credential.unwrap_or_default(),
            ..Default::default()
        })
        .collect();

    let config = RTCConfiguration {
        ice_servers,
        ..Default::default()
    };

    let pc = api.new_peer_connection(config).await?;
    Ok(Arc::new(pc))
}

// ─── ICE gathering helper ───────────────────────────────────────────────────

async fn wait_for_ice(
    pc: &Arc<RTCPeerConnection>,
    timeout_secs: u64,
) {
    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    let tx = Arc::new(std::sync::Mutex::new(Some(tx)));
    pc.on_ice_gathering_state_change(Box::new(move |state| {
        if state == webrtc::ice_transport::ice_gatherer_state::RTCIceGathererState::Complete {
            if let Some(t) = tx.lock().unwrap().take() {
                let _ = t.send(());
            }
        }
        Box::pin(async {})
    }));
    let _ = tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), rx).await;
}

// ─── SDP exchange helper ────────────────────────────────────────────────────

async fn exchange_sdp(
    pc: &Arc<RTCPeerConnection>,
    offer_sdp: String,
) -> Result<SdpAnswer, ApiError> {
    let sdp_offer = RTCSessionDescription::offer(offer_sdp).map_err(|e| {
        warn!("Invalid SDP offer: {e}");
        ApiError::invalid_sdp()
    })?;

    pc.set_remote_description(sdp_offer).await.map_err(|e| {
        warn!("set_remote_description failed: {e}");
        ApiError::internal("set_remote_description failed")
    })?;

    let answer = pc.create_answer(None).await.map_err(|e| {
        warn!("create_answer failed: {e}");
        ApiError::internal("create_answer failed")
    })?;

    pc.set_local_description(answer).await.map_err(|e| {
        warn!("set_local_description failed: {e}");
        ApiError::internal("set_local_description failed")
    })?;

    wait_for_ice(pc, 10).await;

    let local_desc = pc
        .local_description()
        .await
        .ok_or_else(|| ApiError::internal("local_description unavailable after ICE gathering"))?;

    Ok(SdpAnswer {
        sdp: local_desc.sdp,
        sdp_type: "answer".to_string(),
    })
}

// ─── Publisher readiness helper ─────────────────────────────────────────────

/// Wait for a publisher's `on_track` callback to fire and populate the video
/// codec.  This avoids a race condition where the subscriber creates tracks
/// with a default/empty codec before the publisher's RTP has actually started
/// flowing.
///
/// Returns once `publisher.video_codec` is `Some(_)` or after `timeout_secs`
/// (whichever comes first).  A timeout is NOT an error — it just means we
/// fall through to the default VP8 codec (which may work, but might produce
/// a black frame until a keyframe arrives).
async fn wait_for_publisher_ready(publisher: &Publisher, timeout_secs: u64) {
    if publisher.video_codec.read().unwrap().is_some() {
        return; // Already ready.
    }
    let max_wait = std::time::Duration::from_secs(timeout_secs);
    let poll_interval = std::time::Duration::from_millis(100);
    let start = std::time::Instant::now();
    while publisher.video_codec.read().unwrap().is_none() {
        if start.elapsed() > max_wait {
            warn!(
                "Timed out waiting for publisher '{}' video codec ({}s)",
                publisher.peer_id, timeout_secs
            );
            break;
        }
        tokio::time::sleep(poll_interval).await;
    }
    if publisher.video_codec.read().unwrap().is_some() {
        info!(
            "Publisher '{}' video codec ready after {:?}",
            publisher.peer_id,
            start.elapsed()
        );
    }
}

// ─── Fan-out helpers ────────────────────────────────────────────────────────

fn spawn_fanout_task(
    mut rx: broadcast::Receiver<webrtc::rtp::packet::Packet>,
    track: Arc<TrackLocalStaticRTP>,
    cancel: CancellationToken,
    label: &'static str,
) {
    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = cancel.cancelled() => {
                    info!("{label} fanout task cancelled");
                    break;
                }
                result = rx.recv() => {
                    match result {
                        Ok(pkt) => {
                            if let Err(e) = track.write_rtp(&pkt).await {
                                warn!("{label} write_rtp error: {e}");
                                break;
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(n)) => {
                            warn!("{label} subscriber lagged, skipped {n} packets");
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            info!("{label} publisher closed channel");
                            break;
                        }
                    }
                }
            }
        }
    });
}

/// Dynamic fan-out variant that accepts an owned String label.
/// Used for conference mode where labels are built at runtime.
fn spawn_fanout_task_dynamic(
    mut rx: broadcast::Receiver<webrtc::rtp::packet::Packet>,
    track: Arc<TrackLocalStaticRTP>,
    cancel: CancellationToken,
    label: String,
) {
    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = cancel.cancelled() => {
                    info!("{label} fanout task cancelled");
                    break;
                }
                result = rx.recv() => {
                    match result {
                        Ok(pkt) => {
                            if let Err(e) = track.write_rtp(&pkt).await {
                                warn!("{label} write_rtp error: {e}");
                                break;
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(n)) => {
                            warn!("{label} subscriber lagged, skipped {n} packets");
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            info!("{label} publisher closed channel");
                            break;
                        }
                    }
                }
            }
        }
    });
}

// ─── Shared on_track setup ──────────────────────────────────────────────────

/// Configure the on_track handler for a publisher. If `is_screen` is true,
/// incoming video RTP is routed to `screen_tx` instead of `video_tx`.
fn setup_publisher_on_track(
    pc: &Arc<RTCPeerConnection>,
    publisher: &Arc<Publisher>,
    room_id: &str,
    is_screen: bool,
) {
    let pub_clone = publisher.clone();
    let rid = room_id.to_string();
    pc.on_track(Box::new(move |track, _receiver, _transceiver| {
        let publisher = pub_clone.clone();
        let rid = rid.clone();

        Box::pin(async move {
            let kind = track.kind();
            info!(
                "Room '{rid}' — track received: kind={kind}, ssrc={}, screen={is_screen}",
                track.ssrc()
            );

            if kind == RTPCodecType::Video {
                if is_screen {
                    *publisher.screen_codec.write().unwrap() =
                        Some(track.codec().capability.clone());
                    publisher
                        .screen_ssrc
                        .store(track.ssrc() as u64, Ordering::Relaxed);

                    let tx = publisher.screen_tx.clone();
                    tokio::spawn(async move {
                        loop {
                            match track.read_rtp().await {
                                Ok((pkt, _)) => { let _ = tx.send(pkt); }
                                Err(e) => {
                                    warn!("RTP read error (screen): {e}");
                                    break;
                                }
                            }
                        }
                    });
                } else {
                    *publisher.video_codec.write().unwrap() =
                        Some(track.codec().capability.clone());
                    publisher
                        .video_ssrc
                        .store(track.ssrc() as u64, Ordering::Relaxed);

                    let tx = publisher.video_tx.clone();
                    tokio::spawn(async move {
                        loop {
                            match track.read_rtp().await {
                                Ok((pkt, _)) => { let _ = tx.send(pkt); }
                                Err(e) => {
                                    warn!("RTP read error (video): {e}");
                                    break;
                                }
                            }
                        }
                    });
                }
            } else {
                *publisher.audio_codec.write().unwrap() =
                    Some(track.codec().capability.clone());

                let tx = publisher.audio_tx.clone();
                tokio::spawn(async move {
                    loop {
                        match track.read_rtp().await {
                            Ok((pkt, _)) => { let _ = tx.send(pkt); }
                            Err(e) => {
                                warn!("RTP read error (audio): {e}");
                                break;
                            }
                        }
                    }
                });
            }
        })
    }));
}

/// Spawn a periodic PLI sender for a publisher (covers both camera and
/// screen SSRCs).
fn spawn_pli_sender(publisher: &Arc<Publisher>) {
    let pub_clone = publisher.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(3));
        loop {
            interval.tick().await;
            // PLI for camera video.
            let ssrc = pub_clone.video_ssrc.load(Ordering::Relaxed);
            if ssrc != 0 {
                let pli = PictureLossIndication {
                    sender_ssrc: 0,
                    media_ssrc: ssrc as u32,
                };
                if let Err(e) = pub_clone.pc.write_rtcp(&[Box::new(pli)]).await {
                    warn!("PLI send error (video): {e}");
                    break;
                }
            }
            // PLI for screen share.
            let screen_ssrc = pub_clone.screen_ssrc.load(Ordering::Relaxed);
            if screen_ssrc != 0 {
                let pli = PictureLossIndication {
                    sender_ssrc: 0,
                    media_ssrc: screen_ssrc as u32,
                };
                if let Err(e) = pub_clone.pc.write_rtcp(&[Box::new(pli)]).await {
                    warn!("PLI send error (screen): {e}");
                    break;
                }
            }
        }
    });
}

// ─── POST /sfu/publish ──────────────────────────────────────────────────────

pub async fn sfu_publish(
    State(state): State<Arc<crate::AppState>>,
    headers: HeaderMap,
    Json(offer): Json<SdpOffer>,
) -> Result<Json<SdpAnswer>, ApiError> {
    // 1. Verify JWT token.
    let token_str = extract_bearer_token(&headers)?;

    let claims = crate::auth::verify_token(&state.jwt_secret, token_str)
        .map_err(|e| match e.kind() {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => ApiError::token_expired(),
            _ => ApiError::token_invalid(),
        })?;

    if claims.role != "publish" && claims.role != "call" {
        return Err(ApiError::role_insufficient(&claims.role));
    }

    let room_id = claims.room_id.clone();
    let peer_id = claims.sub.clone();
    let is_screen = offer.screen;

    // 2. Look up the room.
    let room = {
        let rooms = state.rooms.read().unwrap();
        rooms.get(&room_id).cloned()
    };
    let room = room.ok_or_else(|| {
        warn!("sfu_publish: room '{room_id}' not found");
        ApiError::room_not_found(&room_id)
    })?;

    // 3. Screen shares get a separate peer_id suffix so they don't
    //    collide with the camera publisher entry.
    let effective_peer_id = if is_screen {
        format!("{peer_id}-screen")
    } else {
        peer_id.clone()
    };

    // Screen shares bypass the max_publishers limit — they are additive.
    if !is_screen && !room.can_publish() {
        warn!("sfu_publish: room '{room_id}' is full");
        return Err(ApiError::room_full(&room_id));
    }

    // 4. Create PeerConnection (using dynamic ICE config).
    let pc = create_peer_connection(&state.config).await.map_err(|e| {
        warn!("sfu_publish: failed to create PeerConnection: {e}");
        ApiError::peer_connection_failed()
    })?;

    // 5. Create Publisher.
    let publisher = if is_screen {
        Arc::new(Publisher::new_screen(effective_peer_id.clone(), pc.clone()))
    } else {
        Arc::new(Publisher::new(effective_peer_id.clone(), pc.clone()))
    };

    // 6. on_track — forward incoming RTP to broadcast channels.
    setup_publisher_on_track(&pc, &publisher, &room_id, is_screen);

    // 7. on_peer_connection_state_change — remove publisher on disconnect.
    {
        let room_clone = room.clone();
        let pid = effective_peer_id.clone();
        let rid = room_id.clone();
        let state_clone = state.clone();
        pc.on_peer_connection_state_change(Box::new(move |conn_state| {
            let room = room_clone.clone();
            let pid = pid.clone();
            let rid = rid.clone();
            let state = state_clone.clone();
            Box::pin(async move {
                match conn_state {
                    RTCPeerConnectionState::Failed
                    | RTCPeerConnectionState::Disconnected
                    | RTCPeerConnectionState::Closed => {
                        info!("Publisher '{pid}' disconnected from room '{rid}'");
                        room.remove_publisher(&pid);
                        if room.publisher_count() == 0
                            && room.room_type == crate::room::RoomType::Broadcast
                        {
                            let mut rooms = state.rooms.write().unwrap();
                            rooms.remove(&rid);
                            info!("Room '{rid}' removed (no publishers left)");
                        }
                    }
                    _ => {}
                }
            })
        }));
    }

    // 8. SDP exchange.
    let answer = exchange_sdp(&pc, offer.sdp).await?;

    // 9. Add publisher to room.
    if is_screen {
        // Screen shares are inserted directly, bypassing max_publishers.
        let mut pubs = room.publishers.write().unwrap();
        pubs.insert(publisher.peer_id.clone(), publisher.clone());
    } else {
        room.add_publisher(publisher.clone()).map_err(|_| {
            warn!("sfu_publish: room '{room_id}' became full during SDP exchange");
            ApiError::room_full(&room_id)
        })?;
    }

    // 10. Spawn periodic PLI sender.
    spawn_pli_sender(&publisher);

    info!(
        "Room '{room_id}' — publisher '{effective_peer_id}' connected (screen={is_screen})"
    );
    Ok(Json(answer))
}

// ─── POST /sfu/subscribe ────────────────────────────────────────────────────

pub async fn sfu_subscribe(
    State(state): State<Arc<crate::AppState>>,
    headers: HeaderMap,
    Json(offer): Json<SdpOffer>,
) -> Result<Json<SdpAnswer>, ApiError> {
    // 1. Verify JWT token.
    let token_str = extract_bearer_token(&headers)?;

    let claims = crate::auth::verify_token(&state.jwt_secret, token_str)
        .map_err(|e| match e.kind() {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => ApiError::token_expired(),
            _ => ApiError::token_invalid(),
        })?;

    if claims.role != "subscribe" && claims.role != "call" {
        return Err(ApiError::role_insufficient(&claims.role));
    }

    let room_id = claims.room_id.clone();

    // 2. Look up the room and get publishers.
    let room = {
        let rooms = state.rooms.read().unwrap();
        rooms.get(&room_id).cloned()
    };
    let room = room.ok_or_else(|| {
        warn!("sfu_subscribe: room '{room_id}' not found");
        ApiError::room_not_found(&room_id)
    })?;

    let publishers = room.get_publishers();
    if publishers.is_empty() {
        warn!("sfu_subscribe: room '{room_id}' has no publishers");
        return Err(ApiError::no_publisher(&room_id));
    }

    // For broadcast: subscribe to the main (non-screen) publisher.
    // For call: subscribe to the other peer.
    let target_publisher = if room.room_type == crate::room::RoomType::Broadcast {
        publishers
            .iter()
            .find(|p| !p.peer_id.ends_with("-screen"))
            .cloned()
            .or_else(|| publishers.first().cloned())
    } else {
        publishers
            .iter()
            .find(|p| p.peer_id != claims.sub && !p.peer_id.ends_with("-screen"))
            .cloned()
            .or_else(|| publishers.first().cloned())
    };

    let publisher = target_publisher.ok_or_else(|| {
        warn!("sfu_subscribe: no suitable publisher in room '{room_id}'");
        ApiError::no_publisher(&room_id)
    })?;

    // 2b. Wait for publisher's on_track to fire (avoids black-screen race).
    wait_for_publisher_ready(&publisher, 10).await;

    // 3. Get codecs from publisher.
    let video_codec = publisher
        .video_codec.read().unwrap().clone()
        .unwrap_or_else(|| RTCRtpCodecCapability {
            mime_type: MIME_TYPE_VP8.to_string(),
            ..Default::default()
        });
    let audio_codec = publisher
        .audio_codec.read().unwrap().clone()
        .unwrap_or_else(|| RTCRtpCodecCapability {
            mime_type: MIME_TYPE_OPUS.to_string(),
            ..Default::default()
        });

    // 4. Create PeerConnection.
    let pc = create_peer_connection(&state.config).await.map_err(|e| {
        warn!("sfu_subscribe: failed to create PeerConnection: {e}");
        ApiError::peer_connection_failed()
    })?;

    // 5. Create local tracks — camera video + audio.
    let video_track = Arc::new(TrackLocalStaticRTP::new(
        video_codec,
        "video".to_string(),
        "liverelay-cam".to_string(),
    ));
    let audio_track = Arc::new(TrackLocalStaticRTP::new(
        audio_codec,
        "audio".to_string(),
        "liverelay-cam".to_string(),
    ));

    pc.add_track(Arc::clone(&video_track) as Arc<dyn TrackLocal + Send + Sync>)
        .await.map_err(|e| {
            warn!("sfu_subscribe: add_track(video) failed: {e}");
            ApiError::internal("add_track(video) failed")
        })?;
    pc.add_track(Arc::clone(&audio_track) as Arc<dyn TrackLocal + Send + Sync>)
        .await.map_err(|e| {
            warn!("sfu_subscribe: add_track(audio) failed: {e}");
            ApiError::internal("add_track(audio) failed")
        })?;

    // 5b. Screen share track — check if publisher has an inline screen
    //     channel or if there is a dedicated "-screen" publisher.
    let screen_track = {
        let screen_pub = publishers.iter().find(|p| p.peer_id.ends_with("-screen"));
        let has_inline_screen = publisher.has_screen();

        if has_inline_screen || screen_pub.is_some() {
            let source = if has_inline_screen {
                publisher.clone()
            } else {
                screen_pub.unwrap().clone()
            };
            let codec = source.screen_codec.read().unwrap().clone()
                .or_else(|| source.video_codec.read().unwrap().clone())
                .unwrap_or_else(|| RTCRtpCodecCapability {
                    mime_type: MIME_TYPE_VP8.to_string(),
                    ..Default::default()
                });
            let track = Arc::new(TrackLocalStaticRTP::new(
                codec,
                "screen".to_string(),
                "liverelay-screen".to_string(),
            ));
            pc.add_track(Arc::clone(&track) as Arc<dyn TrackLocal + Send + Sync>)
                .await.map_err(|e| {
                    warn!("sfu_subscribe: add_track(screen) failed: {e}");
                    ApiError::internal("add_track(screen) failed")
                })?;
            Some((track, has_inline_screen, screen_pub.cloned()))
        } else {
            None
        }
    };

    // 6. Cancellation token.
    let cancel = CancellationToken::new();

    // 7. Monitor connection state.
    {
        let cancel_clone = cancel.clone();
        let room_clone = room.clone();
        pc.on_peer_connection_state_change(Box::new(
            move |conn_state: RTCPeerConnectionState| {
                let cancel = cancel_clone.clone();
                let room = room_clone.clone();
                Box::pin(async move {
                    info!("subscriber connection state: {conn_state}");
                    match conn_state {
                        RTCPeerConnectionState::Failed
                        | RTCPeerConnectionState::Disconnected
                        | RTCPeerConnectionState::Closed => {
                            cancel.cancel();
                            room.subscriber_count
                                .fetch_sub(1, Ordering::Relaxed);
                        }
                        _ => {}
                    }
                })
            },
        ));
    }

    // 8. SDP exchange.
    let answer = exchange_sdp(&pc, offer.sdp).await?;

    // 9. Subscribe to broadcast channels — camera.
    let video_rx = publisher.video_tx.subscribe();
    let audio_rx = publisher.audio_tx.subscribe();

    // 10. Spawn fan-out tasks.
    spawn_fanout_task(video_rx, Arc::clone(&video_track), cancel.clone(), "video");
    spawn_fanout_task(audio_rx, Arc::clone(&audio_track), cancel.clone(), "audio");

    // 10b. Screen share fan-out.
    if let Some((track, has_inline, screen_pub_opt)) = screen_track {
        let screen_rx = if has_inline {
            publisher.screen_tx.subscribe()
        } else if let Some(ref sp) = screen_pub_opt {
            sp.video_tx.subscribe()
        } else {
            publisher.screen_tx.subscribe()
        };
        spawn_fanout_task(screen_rx, track, cancel.clone(), "screen");
    }

    // 11. Request immediate keyframe.
    let ssrc = publisher.video_ssrc.load(Ordering::Relaxed);
    if ssrc != 0 {
        let pli = PictureLossIndication {
            sender_ssrc: 0,
            media_ssrc: ssrc as u32,
        };
        let _ = publisher.pc.write_rtcp(&[Box::new(pli)]).await;
    }

    // 12. Bump subscriber count.
    let count = room
        .subscriber_count
        .fetch_add(1, Ordering::Relaxed)
        + 1;
    info!("Room '{room_id}' now has {count} subscriber(s)");

    Ok(Json(answer))
}

// ─── POST /sfu/call ─────────────────────────────────────────────────────────

pub async fn sfu_call(
    State(state): State<Arc<crate::AppState>>,
    headers: HeaderMap,
    Json(offer): Json<SdpOffer>,
) -> Result<Json<SdpAnswer>, ApiError> {
    // 1. Verify JWT token.
    let token_str = extract_bearer_token(&headers)?;

    let claims = crate::auth::verify_token(&state.jwt_secret, token_str)
        .map_err(|e| match e.kind() {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => ApiError::token_expired(),
            _ => ApiError::token_invalid(),
        })?;

    if claims.role != "call" {
        return Err(ApiError::role_insufficient(&claims.role));
    }

    let room_id = claims.room_id.clone();
    let peer_id = claims.sub.clone();

    // 2. Look up the room.
    let room = {
        let rooms = state.rooms.read().unwrap();
        rooms.get(&room_id).cloned()
    };
    let room = room.ok_or_else(|| {
        warn!("sfu_call: room '{room_id}' not found");
        ApiError::room_not_found(&room_id)
    })?;

    if room.room_type != crate::room::RoomType::Call {
        warn!("sfu_call: room '{room_id}' is not a call room");
        return Err(ApiError::room_type_mismatch("call", "broadcast"));
    }

    // 3. Create PeerConnection (using dynamic ICE config).
    let pc = create_peer_connection(&state.config).await.map_err(|e| {
        warn!("sfu_call: failed to create PeerConnection: {e}");
        ApiError::peer_connection_failed()
    })?;

    // 4. Create Publisher for this peer (call = each peer publishes).
    let publisher = Arc::new(Publisher::new(peer_id.clone(), pc.clone()));

    // 5. Setup on_track for incoming media.
    setup_publisher_on_track(&pc, &publisher, &room_id, false);

    // 6. Check if the other peer is already in the room — subscribe to them.
    let other_publisher = {
        let pubs = room.get_publishers();
        pubs.into_iter().find(|p| p.peer_id != peer_id)
    };

    let cancel = CancellationToken::new();

    if let Some(other) = &other_publisher {
        // Wait for the other peer's on_track to fire before reading codecs.
        wait_for_publisher_ready(other, 10).await;

        let video_codec = other.video_codec.read().unwrap().clone()
            .unwrap_or_else(|| RTCRtpCodecCapability {
                mime_type: MIME_TYPE_VP8.to_string(),
                ..Default::default()
            });
        let audio_codec = other.audio_codec.read().unwrap().clone()
            .unwrap_or_else(|| RTCRtpCodecCapability {
                mime_type: MIME_TYPE_OPUS.to_string(),
                ..Default::default()
            });

        let video_track = Arc::new(TrackLocalStaticRTP::new(
            video_codec, "video".to_string(), "liverelay".to_string(),
        ));
        let audio_track = Arc::new(TrackLocalStaticRTP::new(
            audio_codec, "audio".to_string(), "liverelay".to_string(),
        ));

        pc.add_track(Arc::clone(&video_track) as Arc<dyn TrackLocal + Send + Sync>)
            .await.map_err(|e| {
                warn!("sfu_call: add_track(video) failed: {e}");
                ApiError::internal("add_track(video) failed")
            })?;
        pc.add_track(Arc::clone(&audio_track) as Arc<dyn TrackLocal + Send + Sync>)
            .await.map_err(|e| {
                warn!("sfu_call: add_track(audio) failed: {e}");
                ApiError::internal("add_track(audio) failed")
            })?;

        let video_rx = other.video_tx.subscribe();
        let audio_rx = other.audio_tx.subscribe();

        spawn_fanout_task(video_rx, video_track, cancel.clone(), "call-video");
        spawn_fanout_task(audio_rx, audio_track, cancel.clone(), "call-audio");

        // Request keyframe from other peer.
        let ssrc = other.video_ssrc.load(Ordering::Relaxed);
        if ssrc != 0 {
            let pli = PictureLossIndication {
                sender_ssrc: 0,
                media_ssrc: ssrc as u32,
            };
            let _ = other.pc.write_rtcp(&[Box::new(pli)]).await;
        }
    }

    // 7. Cleanup on disconnect.
    {
        let cancel_clone = cancel.clone();
        let room_clone = room.clone();
        let pid = peer_id.clone();
        let rid = room_id.clone();
        let state_clone = state.clone();
        pc.on_peer_connection_state_change(Box::new(move |conn_state| {
            let cancel = cancel_clone.clone();
            let room = room_clone.clone();
            let pid = pid.clone();
            let rid = rid.clone();
            let state = state_clone.clone();
            Box::pin(async move {
                match conn_state {
                    RTCPeerConnectionState::Failed
                    | RTCPeerConnectionState::Disconnected
                    | RTCPeerConnectionState::Closed => {
                        cancel.cancel();
                        room.remove_publisher(&pid);
                        info!("Call peer '{pid}' disconnected from room '{rid}'");
                        if room.publisher_count() == 0 {
                            let mut rooms = state.rooms.write().unwrap();
                            rooms.remove(&rid);
                            info!("Call room '{rid}' removed (empty)");
                        }
                    }
                    _ => {}
                }
            })
        }));
    }

    // 8. SDP exchange.
    let answer = exchange_sdp(&pc, offer.sdp).await?;

    // 9. Add publisher to room.
    room.add_publisher(publisher).map_err(|_| {
        warn!("sfu_call: room '{room_id}' is full");
        ApiError::room_full(&room_id)
    })?;

    info!("Call peer '{peer_id}' joined room '{room_id}'");
    Ok(Json(answer))
}

// ═══════════════════════════════════════════════════════════════════════════
// CONFERENCE MODE — N-party multi-peer
// ═══════════════════════════════════════════════════════════════════════════
//
// Architecture:
//
//   Each participant has a single PeerConnection that does BOTH:
//     - Publish: sends their camera (+ optional screen) media upstream.
//     - Subscribe: receives tracks from all currently-present publishers.
//
//   When joining, the server adds send tracks for the joiner's media and
//   receive tracks for every existing publisher. The answer SDP includes
//   a `participants` list so the SDK knows who is already present.
//
//   When a NEW participant joins AFTER you, your SDK opens a lightweight
//   subscribe-only PeerConnection via POST /sfu/conference/subscribe
//   to receive just that newcomer's tracks. This avoids SDP renegotiation
//   on the original PeerConnection.
//
//   Connection count per participant:
//     - 1 main PC (publish + subscribe to everyone present at join time)
//     - +1 PC per newcomer who joins later
//     - Total: 1 + (latecomers) PeerConnections per participant
//

/// Extended answer for conference mode.
#[derive(Serialize)]
pub struct ConferenceAnswer {
    pub sdp: String,
    #[serde(rename = "type")]
    pub sdp_type: String,
    /// Peer IDs of publishers present when you joined (whose tracks are
    /// included in this SDP answer).
    pub participants: Vec<String>,
    /// Your own peer_id (from the JWT `sub` claim).
    pub peer_id: String,
}

/// POST /sfu/conference — join a conference room (publish + subscribe).
pub async fn sfu_conference(
    State(state): State<Arc<crate::AppState>>,
    headers: HeaderMap,
    Json(offer): Json<SdpOffer>,
) -> Result<Json<ConferenceAnswer>, ApiError> {
    let token_str = extract_bearer_token(&headers)?;

    let claims = crate::auth::verify_token(&state.jwt_secret, token_str)
        .map_err(|e| match e.kind() {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => ApiError::token_expired(),
            _ => ApiError::token_invalid(),
        })?;

    if claims.role != "conference" && claims.role != "call" {
        return Err(ApiError::role_insufficient(&claims.role));
    }

    let room_id = claims.room_id.clone();
    let peer_id = claims.sub.clone();

    let room = {
        let rooms = state.rooms.read().unwrap();
        rooms.get(&room_id).cloned()
    };
    let room = room.ok_or_else(|| {
        warn!("sfu_conference: room '{room_id}' not found");
        ApiError::room_not_found(&room_id)
    })?;

    if room.room_type != crate::room::RoomType::Conference {
        return Err(ApiError::room_type_mismatch(
            "conference",
            &format!("{:?}", room.room_type),
        ));
    }

    if !room.can_publish() {
        return Err(ApiError::room_full(&room_id));
    }

    // Get existing publishers (everyone except ourselves).
    let other_publishers = room.get_other_publishers(&peer_id);

    // Create PeerConnection.
    let pc = create_peer_connection(&state.config).await.map_err(|e| {
        warn!("sfu_conference: failed to create PeerConnection: {e}");
        ApiError::peer_connection_failed()
    })?;

    // Create Publisher for this peer.
    let publisher = Arc::new(Publisher::new(peer_id.clone(), pc.clone()));

    // Setup on_track (publish path).
    setup_publisher_on_track(&pc, &publisher, &room_id, false);

    // For each existing publisher, add receive tracks.
    let cancel = CancellationToken::new();
    let mut participant_list: Vec<String> = Vec::new();

    for other in &other_publishers {
        participant_list.push(other.peer_id.clone());

        // Wait for each publisher's codec to be ready.
        wait_for_publisher_ready(other, 10).await;

        let video_codec = other.video_codec.read().unwrap().clone()
            .unwrap_or_else(|| RTCRtpCodecCapability {
                mime_type: MIME_TYPE_VP8.to_string(),
                ..Default::default()
            });
        let audio_codec = other.audio_codec.read().unwrap().clone()
            .unwrap_or_else(|| RTCRtpCodecCapability {
                mime_type: MIME_TYPE_OPUS.to_string(),
                ..Default::default()
            });

        let short_id = &other.peer_id[..8.min(other.peer_id.len())];
        let stream_id = format!("lr-{short_id}");

        let video_track = Arc::new(TrackLocalStaticRTP::new(
            video_codec,
            format!("video-{}", other.peer_id),
            stream_id.clone(),
        ));
        let audio_track = Arc::new(TrackLocalStaticRTP::new(
            audio_codec,
            format!("audio-{}", other.peer_id),
            stream_id,
        ));

        pc.add_track(Arc::clone(&video_track) as Arc<dyn TrackLocal + Send + Sync>)
            .await.map_err(|e| {
                warn!("sfu_conference: add_track(video) failed for {}: {e}", other.peer_id);
                ApiError::internal("add_track(video) failed")
            })?;
        pc.add_track(Arc::clone(&audio_track) as Arc<dyn TrackLocal + Send + Sync>)
            .await.map_err(|e| {
                warn!("sfu_conference: add_track(audio) failed for {}: {e}", other.peer_id);
                ApiError::internal("add_track(audio) failed")
            })?;

        let video_rx = other.video_tx.subscribe();
        let audio_rx = other.audio_tx.subscribe();

        spawn_fanout_task_dynamic(
            video_rx, video_track, cancel.clone(),
            format!("conf-video-{short_id}"),
        );
        spawn_fanout_task_dynamic(
            audio_rx, audio_track, cancel.clone(),
            format!("conf-audio-{short_id}"),
        );

        // Request keyframe.
        let ssrc = other.video_ssrc.load(Ordering::Relaxed);
        if ssrc != 0 {
            let pli = PictureLossIndication {
                sender_ssrc: 0,
                media_ssrc: ssrc as u32,
            };
            let _ = other.pc.write_rtcp(&[Box::new(pli)]).await;
        }
    }

    // Cleanup on disconnect.
    {
        let cancel_clone = cancel.clone();
        let room_clone = room.clone();
        let pid = peer_id.clone();
        let rid = room_id.clone();
        let state_clone = state.clone();
        pc.on_peer_connection_state_change(Box::new(move |conn_state| {
            let cancel = cancel_clone.clone();
            let room = room_clone.clone();
            let pid = pid.clone();
            let rid = rid.clone();
            let state = state_clone.clone();
            Box::pin(async move {
                match conn_state {
                    RTCPeerConnectionState::Failed
                    | RTCPeerConnectionState::Disconnected
                    | RTCPeerConnectionState::Closed => {
                        cancel.cancel();
                        room.remove_publisher(&pid);
                        room.subscriber_count.fetch_sub(1, Ordering::Relaxed);
                        info!("Conference peer '{pid}' disconnected from room '{rid}'");
                        if room.publisher_count() == 0 {
                            let mut rooms = state.rooms.write().unwrap();
                            rooms.remove(&rid);
                            info!("Conference room '{rid}' removed (empty)");
                        }
                    }
                    _ => {}
                }
            })
        }));
    }

    // SDP exchange.
    let answer = exchange_sdp(&pc, offer.sdp).await?;

    // Add publisher to room.
    room.add_publisher(publisher.clone()).map_err(|_| {
        warn!("sfu_conference: room '{room_id}' is full");
        ApiError::room_full(&room_id)
    })?;

    // Bump subscriber count.
    room.subscriber_count.fetch_add(1, Ordering::Relaxed);

    // Start PLI sender.
    spawn_pli_sender(&publisher);

    info!(
        "Conference peer '{peer_id}' joined room '{room_id}' — {} other(s) present",
        other_publishers.len()
    );

    Ok(Json(ConferenceAnswer {
        sdp: answer.sdp,
        sdp_type: answer.sdp_type,
        participants: participant_list,
        peer_id,
    }))
}

// ─── POST /sfu/conference/subscribe ─────────────────────────────────────────

#[derive(Deserialize)]
pub struct ConferenceSubscribeRequest {
    pub sdp: String,
    #[serde(rename = "type")]
    #[allow(dead_code)]
    pub sdp_type: String,
    /// The peer_id of the publisher to subscribe to.
    pub target_peer_id: String,
}

pub async fn sfu_conference_subscribe(
    State(state): State<Arc<crate::AppState>>,
    headers: HeaderMap,
    Json(req): Json<ConferenceSubscribeRequest>,
) -> Result<Json<SdpAnswer>, ApiError> {
    let token_str = extract_bearer_token(&headers)?;

    let claims = crate::auth::verify_token(&state.jwt_secret, token_str)
        .map_err(|e| match e.kind() {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => ApiError::token_expired(),
            _ => ApiError::token_invalid(),
        })?;

    if claims.role != "conference" && claims.role != "call" {
        return Err(ApiError::role_insufficient(&claims.role));
    }

    let room_id = claims.room_id.clone();

    let room = {
        let rooms = state.rooms.read().unwrap();
        rooms.get(&room_id).cloned()
    };
    let room = room.ok_or_else(|| ApiError::room_not_found(&room_id))?;

    let target = {
        let pubs = room.publishers.read().unwrap();
        pubs.get(&req.target_peer_id).cloned()
    };
    let target = target.ok_or_else(|| {
        ApiError::not_found(format!(
            "Publisher '{}' not found in room '{room_id}'",
            req.target_peer_id
        ))
    })?;

    // Wait for target publisher's codec to be ready.
    wait_for_publisher_ready(&target, 10).await;

    let pc = create_peer_connection(&state.config).await.map_err(|e| {
        warn!("sfu_conference_subscribe: PC creation failed: {e}");
        ApiError::peer_connection_failed()
    })?;

    let video_codec = target.video_codec.read().unwrap().clone()
        .unwrap_or_else(|| RTCRtpCodecCapability {
            mime_type: MIME_TYPE_VP8.to_string(),
            ..Default::default()
        });
    let audio_codec = target.audio_codec.read().unwrap().clone()
        .unwrap_or_else(|| RTCRtpCodecCapability {
            mime_type: MIME_TYPE_OPUS.to_string(),
            ..Default::default()
        });

    let short_id = &target.peer_id[..8.min(target.peer_id.len())];
    let stream_id = format!("lr-{short_id}");

    let video_track = Arc::new(TrackLocalStaticRTP::new(
        video_codec,
        format!("video-{}", target.peer_id),
        stream_id.clone(),
    ));
    let audio_track = Arc::new(TrackLocalStaticRTP::new(
        audio_codec,
        format!("audio-{}", target.peer_id),
        stream_id,
    ));

    pc.add_track(Arc::clone(&video_track) as Arc<dyn TrackLocal + Send + Sync>)
        .await.map_err(|e| ApiError::internal(format!("add_track(video): {e}")))?;
    pc.add_track(Arc::clone(&audio_track) as Arc<dyn TrackLocal + Send + Sync>)
        .await.map_err(|e| ApiError::internal(format!("add_track(audio): {e}")))?;

    let cancel = CancellationToken::new();

    {
        let cancel_clone = cancel.clone();
        let room_clone = room.clone();
        pc.on_peer_connection_state_change(Box::new(move |conn_state| {
            let cancel = cancel_clone.clone();
            let room = room_clone.clone();
            Box::pin(async move {
                match conn_state {
                    RTCPeerConnectionState::Failed
                    | RTCPeerConnectionState::Disconnected
                    | RTCPeerConnectionState::Closed => {
                        cancel.cancel();
                        room.subscriber_count.fetch_sub(1, Ordering::Relaxed);
                    }
                    _ => {}
                }
            })
        }));
    }

    let answer = exchange_sdp(&pc, req.sdp).await?;

    let video_rx = target.video_tx.subscribe();
    let audio_rx = target.audio_tx.subscribe();

    spawn_fanout_task(video_rx, video_track, cancel.clone(), "conf-sub-video");
    spawn_fanout_task(audio_rx, audio_track, cancel.clone(), "conf-sub-audio");

    let ssrc = target.video_ssrc.load(Ordering::Relaxed);
    if ssrc != 0 {
        let pli = PictureLossIndication {
            sender_ssrc: 0,
            media_ssrc: ssrc as u32,
        };
        let _ = target.pc.write_rtcp(&[Box::new(pli)]).await;
    }

    room.subscriber_count.fetch_add(1, Ordering::Relaxed);

    info!(
        "Conference subscriber for peer '{}' in room '{room_id}'",
        req.target_peer_id
    );

    Ok(Json(answer))
}
