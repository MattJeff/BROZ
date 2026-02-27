use std::sync::Arc;

use chrono::Utc;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use socketioxide::extract::{Data, SocketRef, State};
use uuid::Uuid;

use crate::events::publisher;
use crate::matching::{algorithm, history, queue};
use crate::models::NewMatchSession;
use crate::schema::match_sessions;
use crate::AppState;

// ---------------------------------------------------------------------------
// Payload types for Socket.IO events
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct JoinQueuePayload {
    pub display_name: String,
    pub bio: Option<String>,
    pub age: i32,
    pub country: Option<String>,
    pub kinks: Vec<String>,
    pub profile_photo_url: Option<String>,
    pub filters: algorithm::MatchFilters,
}

#[derive(Debug, Serialize)]
pub struct MatchFoundPayload {
    pub match_id: Uuid,
    pub partner: PartnerInfo,
    pub is_initiator: bool,
}

#[derive(Debug, Serialize)]
pub struct PartnerInfo {
    pub user_id: Uuid,
    pub display_name: String,
    pub bio: Option<String>,
    pub age: i32,
    pub country: Option<String>,
    pub kinks: Vec<String>,
    pub profile_photo_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct EndCallPayload {
    pub match_id: Uuid,
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ChatMessagePayload {
    pub match_id: Uuid,
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct ChatMessageOut {
    pub match_id: Uuid,
    pub sender_id: Uuid,
    pub content: String,
    pub timestamp: String,
}

// Accept any JSON — the frontend sends flat { type, sdp, match_id } or { type, candidate, match_id }
// We just extract match_id and relay everything else.
pub type WebRtcSignalPayload = serde_json::Value;

#[derive(Debug, Serialize)]
pub struct QueueStatusPayload {
    pub queue_size: u64,
}

#[derive(Debug, Serialize)]
pub struct ErrorPayload {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct SendLikePayload {
    pub match_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct SendFollowRequestPayload {
    pub target_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct RespondFollowRequestPayload {
    pub follower_id: Uuid,
    pub accepted: bool,
}

// ---------------------------------------------------------------------------
// Connection handler
// ---------------------------------------------------------------------------

pub async fn on_connect(socket: SocketRef, state: State<Arc<AppState>>) {
    // Authenticate via query param ?token=xxx
    let user_id = match authenticate_socket(&socket, &state) {
        Ok(id) => id,
        Err(msg) => {
            tracing::warn!(error = %msg, "socket auth failed");
            let _ = socket.emit(
                "error",
                &ErrorPayload {
                    code: "AUTH_FAILED".into(),
                    message: msg,
                },
            );
            socket.disconnect().ok();
            return;
        }
    };

    // Store user_id in socket extensions
    socket.extensions.insert(user_id);

    // CRITICAL: Register event handlers FIRST, before any async operations.
    // The client receives the CONNECT_ACK and may emit events (e.g. join-queue)
    // before our async Redis calls complete. If handlers aren't registered yet,
    // those events are silently dropped by socketioxide.
    socket.on("join-queue", on_join_queue);
    socket.on("next-match", on_next_match);
    socket.on("end-call", on_end_call);
    socket.on("send-like", on_send_like);
    socket.on("send-follow-request", on_send_follow_request);
    socket.on("respond-follow-request", on_respond_follow_request);
    socket.on("chat-message", on_chat_message);
    socket.on("webrtc-signal", on_webrtc_signal);
    socket.on("leave-queue", on_leave_queue);
    socket.on_disconnect(on_disconnect);

    // Join user-specific room for targeted messages
    let user_room = format!("user:{user_id}");
    socket.join(user_room).ok();

    tracing::info!(user_id = %user_id, sid = %socket.id, "socket connected");

    // Set presence in Redis (async — handlers already registered above)
    let presence_key = format!("presence:{user_id}");
    let _ = state.redis.set(&presence_key, &socket.id.to_string(), 3600).await;
    let _ = state.redis.set(&format!("online:match:{user_id}"), "1", 3600).await;
    let _ = state.redis.set(&format!("online:{user_id}"), "1", 3600).await;

    // Emit connected acknowledgment
    let _ = socket.emit("connected", &serde_json::json!({ "user_id": user_id }));
}

// ---------------------------------------------------------------------------
// Event: join-queue
// ---------------------------------------------------------------------------

async fn on_join_queue(
    socket: SocketRef,
    state: State<Arc<AppState>>,
    Data(raw): Data<serde_json::Value>,
) {
    let user_id = match get_user_id(&socket) {
        Some(id) => id,
        None => return,
    };

    tracing::info!(user_id = %user_id, raw = %raw, "join-queue raw payload received");

    let payload: JoinQueuePayload = match serde_json::from_value(raw) {
        Ok(p) => p,
        Err(e) => {
            tracing::error!(user_id = %user_id, error = %e, "join-queue deserialization failed");
            let _ = socket.emit(
                "error",
                &ErrorPayload {
                    code: "INVALID_PAYLOAD".into(),
                    message: format!("Invalid join-queue payload: {e}"),
                },
            );
            return;
        }
    };

    // Check if already in queue
    if queue::is_in_queue(&state.redis, &user_id).await {
        let _ = socket.emit(
            "error",
            &ErrorPayload {
                code: "ALREADY_IN_QUEUE".into(),
                message: "You are already in the queue".into(),
            },
        );
        return;
    }

    // Check if already in a match
    if queue::get_user_active_match(&state.redis, &user_id).await.is_some() {
        let _ = socket.emit(
            "error",
            &ErrorPayload {
                code: "ALREADY_IN_MATCH".into(),
                message: "You are already in a match. End the current call first.".into(),
            },
        );
        return;
    }

    let queue_user = algorithm::QueueUser {
        user_id,
        display_name: payload.display_name,
        bio: payload.bio,
        age: payload.age,
        country: payload.country,
        kinks: payload.kinks,
        profile_photo_url: payload.profile_photo_url,
        filters: payload.filters,
        joined_at: Utc::now().timestamp_millis(),
    };

    if let Err(e) = queue::add_to_queue(&state.redis, &queue_user).await {
        tracing::error!(error = %e, "failed to add user to queue");
        let _ = socket.emit(
            "error",
            &ErrorPayload {
                code: "QUEUE_ERROR".into(),
                message: "Failed to join queue".into(),
            },
        );
        return;
    }

    let queue_size = queue::get_queue_size(&state.redis).await;
    let _ = socket.emit(
        "queue-joined",
        &QueueStatusPayload { queue_size },
    );

    tracing::info!(user_id = %user_id, queue_size = queue_size, "user joined queue");

    // Try to find a match
    try_match(&socket, &state).await;
}

// ---------------------------------------------------------------------------
// Event: leave-queue
// ---------------------------------------------------------------------------

async fn on_leave_queue(socket: SocketRef, state: State<Arc<AppState>>) {
    let user_id = match get_user_id(&socket) {
        Some(id) => id,
        None => return,
    };

    if let Err(e) = queue::remove_from_queue(&state.redis, &user_id).await {
        tracing::error!(error = %e, "failed to remove user from queue");
    }

    let _ = socket.emit("queue-left", &serde_json::json!({}));
    tracing::info!(user_id = %user_id, "user left queue");
}

// ---------------------------------------------------------------------------
// Event: next-match (end current, rejoin queue)
// ---------------------------------------------------------------------------

async fn on_next_match(
    socket: SocketRef,
    state: State<Arc<AppState>>,
    Data(payload): Data<JoinQueuePayload>,
) {
    let user_id = match get_user_id(&socket) {
        Some(id) => id,
        None => return,
    };

    // End current match if any
    if let Some(match_id) = queue::get_user_active_match(&state.redis, &user_id).await {
        end_match_session(&state, &socket, &match_id, &user_id, "next").await;
    }

    // Remove from queue in case they are still there
    let _ = queue::remove_from_queue(&state.redis, &user_id).await;

    // Rejoin queue with new filters
    let queue_user = algorithm::QueueUser {
        user_id,
        display_name: payload.display_name,
        bio: payload.bio,
        age: payload.age,
        country: payload.country,
        kinks: payload.kinks,
        profile_photo_url: payload.profile_photo_url,
        filters: payload.filters,
        joined_at: Utc::now().timestamp_millis(),
    };

    if let Err(e) = queue::add_to_queue(&state.redis, &queue_user).await {
        tracing::error!(error = %e, "failed to rejoin queue");
        let _ = socket.emit(
            "error",
            &ErrorPayload {
                code: "QUEUE_ERROR".into(),
                message: "Failed to rejoin queue".into(),
            },
        );
        return;
    }

    let queue_size = queue::get_queue_size(&state.redis).await;
    let _ = socket.emit("queue-joined", &QueueStatusPayload { queue_size });

    try_match(&socket, &state).await;
}

// ---------------------------------------------------------------------------
// Event: end-call
// ---------------------------------------------------------------------------

async fn on_end_call(
    socket: SocketRef,
    state: State<Arc<AppState>>,
    Data(payload): Data<EndCallPayload>,
) {
    let user_id = match get_user_id(&socket) {
        Some(id) => id,
        None => return,
    };

    let reason = payload.reason.unwrap_or_else(|| "user_ended".into());
    end_match_session(&state, &socket, &payload.match_id, &user_id, &reason).await;

    let _ = socket.emit("call-ended", &serde_json::json!({ "match_id": payload.match_id }));
}

// ---------------------------------------------------------------------------
// Event: send-like
// ---------------------------------------------------------------------------

async fn on_send_like(
    socket: SocketRef,
    state: State<Arc<AppState>>,
    Data(payload): Data<SendLikePayload>,
) {
    let user_id = match get_user_id(&socket) {
        Some(id) => id,
        None => return,
    };

    let match_id = payload.match_id;

    // Get partner from active pair
    let partner_id = match queue::get_partner(&state.redis, &match_id, &user_id).await {
        Some(id) => id,
        None => {
            let _ = socket.emit(
                "error",
                &ErrorPayload {
                    code: "NOT_IN_MATCH".into(),
                    message: "You are not in this match".into(),
                },
            );
            return;
        }
    };

    // Call user service to create the like
    let client = reqwest::Client::new();
    let like_url = format!("{}/likes", state.config.user_service_url);
    let res = client
        .post(&like_url)
        .header("Authorization", format!("Bearer {}", get_user_token(&socket)))
        .json(&serde_json::json!({
            "liked_id": partner_id,
            "match_session_id": match_id,
        }))
        .send()
        .await;

    match res {
        Ok(r) if r.status().is_success() => {
            let _ = socket.emit("like-sent", &serde_json::json!({ "target_id": partner_id }));
            // Track like in session history
            history::increment_session_likes(&state.redis, &match_id).await;
            // Notify partner
            let partner_room = format!("user:{partner_id}");
            let _ = socket.to(partner_room).emit(
                "like-received",
                &serde_json::json!({
                    "from_user_id": user_id,
                    "match_id": match_id,
                }),
            );
        }
        Ok(r) => {
            tracing::warn!(status = %r.status(), "like request failed");
            let _ = socket.emit(
                "error",
                &ErrorPayload {
                    code: "LIKE_FAILED".into(),
                    message: "Failed to send like".into(),
                },
            );
        }
        Err(e) => {
            tracing::error!(error = %e, "like request error");
            let _ = socket.emit(
                "error",
                &ErrorPayload {
                    code: "LIKE_FAILED".into(),
                    message: "Failed to send like".into(),
                },
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Event: send-follow-request
// ---------------------------------------------------------------------------

async fn on_send_follow_request(
    socket: SocketRef,
    state: State<Arc<AppState>>,
    Data(payload): Data<SendFollowRequestPayload>,
) {
    let user_id = match get_user_id(&socket) {
        Some(id) => id,
        None => return,
    };

    let client = reqwest::Client::new();
    let follow_url = format!("{}/follows/{}", state.config.user_service_url, payload.target_id);
    let res = client
        .post(&follow_url)
        .header("Authorization", format!("Bearer {}", get_user_token(&socket)))
        .send()
        .await;

    match res {
        Ok(r) if r.status().is_success() => {
            let _ = socket.emit(
                "follow-request-sent",
                &serde_json::json!({ "target_id": payload.target_id }),
            );
            // Track follow in session history (if user is in an active match)
            if let Some(match_id) = queue::get_user_active_match(&state.redis, &user_id).await {
                history::set_session_follow(&state.redis, &match_id).await;
            }
            // Notify target
            let target_room = format!("user:{}", payload.target_id);
            let _ = socket.to(target_room).emit(
                "follow-request-received",
                &serde_json::json!({ "from_user_id": user_id }),
            );
        }
        Ok(r) => {
            let status = r.status();
            let body = r.text().await.unwrap_or_default();
            tracing::warn!(status = %status, body = %body, "follow request failed");
            let _ = socket.emit(
                "error",
                &ErrorPayload {
                    code: "FOLLOW_FAILED".into(),
                    message: "Failed to send follow request".into(),
                },
            );
        }
        Err(e) => {
            tracing::error!(error = %e, "follow request error");
            let _ = socket.emit(
                "error",
                &ErrorPayload {
                    code: "FOLLOW_FAILED".into(),
                    message: "Failed to send follow request".into(),
                },
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Event: respond-follow-request
// ---------------------------------------------------------------------------

async fn on_respond_follow_request(
    socket: SocketRef,
    state: State<Arc<AppState>>,
    Data(payload): Data<RespondFollowRequestPayload>,
) {
    let user_id = match get_user_id(&socket) {
        Some(id) => id,
        None => return,
    };

    let client = reqwest::Client::new();
    let action = if payload.accepted { "accept" } else { "decline" };
    let follow_url = format!(
        "{}/follows/{}/{}",
        state.config.user_service_url, payload.follower_id, action
    );
    let res = client
        .put(&follow_url)
        .header("Authorization", format!("Bearer {}", get_user_token(&socket)))
        .send()
        .await;

    match res {
        Ok(r) if r.status().is_success() => {
            let _ = socket.emit(
                "follow-response-sent",
                &serde_json::json!({
                    "follower_id": payload.follower_id,
                    "accepted": payload.accepted,
                }),
            );
            // Notify follower
            let follower_room = format!("user:{}", payload.follower_id);
            let event_name = if payload.accepted {
                "follow-request-accepted"
            } else {
                "follow-request-declined"
            };
            let _ = socket.to(follower_room).emit(
                event_name,
                &serde_json::json!({ "by_user_id": user_id }),
            );
        }
        Ok(r) => {
            let status = r.status();
            tracing::warn!(status = %status, "follow response failed");
            let _ = socket.emit(
                "error",
                &ErrorPayload {
                    code: "FOLLOW_RESPONSE_FAILED".into(),
                    message: "Failed to respond to follow request".into(),
                },
            );
        }
        Err(e) => {
            tracing::error!(error = %e, "follow response error");
            let _ = socket.emit(
                "error",
                &ErrorPayload {
                    code: "FOLLOW_RESPONSE_FAILED".into(),
                    message: "Failed to respond to follow request".into(),
                },
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Event: chat-message
// ---------------------------------------------------------------------------

async fn on_chat_message(
    socket: SocketRef,
    state: State<Arc<AppState>>,
    Data(payload): Data<ChatMessagePayload>,
) {
    let user_id = match get_user_id(&socket) {
        Some(id) => id,
        None => return,
    };

    // Get partner
    let partner_id = match queue::get_partner(&state.redis, &payload.match_id, &user_id).await {
        Some(id) => id,
        None => {
            let _ = socket.emit(
                "error",
                &ErrorPayload {
                    code: "NOT_IN_MATCH".into(),
                    message: "You are not in this match".into(),
                },
            );
            return;
        }
    };

    // Track message in session history
    history::increment_session_msgs(&state.redis, &payload.match_id).await;

    let msg = ChatMessageOut {
        match_id: payload.match_id,
        sender_id: user_id,
        content: payload.content,
        timestamp: Utc::now().to_rfc3339(),
    };

    // Relay to partner
    let partner_room = format!("user:{partner_id}");
    let _ = socket.to(partner_room).emit("chat-message", &msg);
}

// ---------------------------------------------------------------------------
// Event: webrtc-signal
// ---------------------------------------------------------------------------

async fn on_webrtc_signal(
    socket: SocketRef,
    state: State<Arc<AppState>>,
    Data(payload): Data<WebRtcSignalPayload>,
) {
    let user_id = match get_user_id(&socket) {
        Some(id) => id,
        None => return,
    };

    // Extract match_id from the flat JSON payload
    let match_id = match payload
        .get("match_id")
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
    {
        Some(id) => id,
        None => {
            tracing::warn!("webrtc-signal missing match_id");
            return;
        }
    };

    // Get partner
    let partner_id = match queue::get_partner(&state.redis, &match_id, &user_id).await {
        Some(id) => id,
        None => {
            let _ = socket.emit(
                "error",
                &ErrorPayload {
                    code: "NOT_IN_MATCH".into(),
                    message: "You are not in this match".into(),
                },
            );
            return;
        }
    };

    // Relay the entire payload + add sender_id
    let mut signal_out = payload.clone();
    if let Some(obj) = signal_out.as_object_mut() {
        obj.insert("sender_id".to_string(), serde_json::json!(user_id));
    }

    let partner_room = format!("user:{partner_id}");
    let _ = socket.to(partner_room).emit("webrtc-signal", &signal_out);
}

// ---------------------------------------------------------------------------
// Disconnect handler
// ---------------------------------------------------------------------------

async fn on_disconnect(socket: SocketRef, state: State<Arc<AppState>>) {
    let user_id = match get_user_id(&socket) {
        Some(id) => id,
        None => return,
    };

    tracing::info!(user_id = %user_id, sid = %socket.id, "socket disconnected");

    // Remove from queue
    let _ = queue::remove_from_queue(&state.redis, &user_id).await;

    // End active match if any
    if let Some(match_id) = queue::get_user_active_match(&state.redis, &user_id).await {
        end_match_session(&state, &socket, &match_id, &user_id, "disconnect").await;
    }

    // Remove presence
    let presence_key = format!("presence:{user_id}");
    let _ = state.redis.del(&presence_key).await;

    // Remove matching presence key
    let _ = state.redis.del(&format!("online:match:{user_id}")).await;

    // Check if messaging service is still connected
    let msg_still_online = state.redis.exists(&format!("online:msg:{user_id}")).await.unwrap_or(false);
    if !msg_still_online {
        let _ = state.redis.del(&format!("online:{user_id}")).await;
    }
}

// ---------------------------------------------------------------------------
// Match-finding algorithm
// ---------------------------------------------------------------------------

async fn try_match(socket: &SocketRef, state: &Arc<AppState>) {
    let user_id = match get_user_id(socket) {
        Some(id) => id,
        None => return,
    };

    // Per-user lock to prevent duplicate matching attempts for the same user
    let lock_key = format!("matching:lock:{user_id}");
    if !state.redis.set_nx(&lock_key, "1", 3).await.unwrap_or(false) {
        let _ = socket.emit("searching", &serde_json::json!({ "status": "searching" }));
        return;
    }

    // Run matching logic, then always release the lock
    try_match_inner(socket, state, &user_id).await;

    // Release lock
    let _ = state.redis.del(&lock_key).await;
}

async fn try_match_inner(socket: &SocketRef, state: &Arc<AppState>, user_id: &Uuid) {
    let queue_users = match queue::get_queue_users(&state.redis).await {
        Ok(users) => users,
        Err(e) => {
            tracing::error!(error = %e, "failed to get queue users");
            return;
        }
    };

    // Find the current user in the queue
    let current_user = match queue_users.iter().find(|u| u.user_id == *user_id) {
        Some(u) => u.clone(),
        None => return, // User not in queue anymore
    };

    // Determine current user's phase based on wait time
    let now_ms = Utc::now().timestamp_millis();
    let wait_ms_a = (now_ms - current_user.joined_at).max(0);
    let phase_a = algorithm::MatchPhase::from_wait_ms(wait_ms_a);

    // Collect candidates (everyone except self)
    let candidates: Vec<&algorithm::QueueUser> = queue_users
        .iter()
        .filter(|u| u.user_id != *user_id)
        .collect();
    let candidate_ids: Vec<Uuid> = candidates.iter().map(|c| c.user_id).collect();

    // Batch Redis lookups: 2 calls instead of 2N
    let cooldowns = queue::has_cooldowns_batch(&state.redis, user_id, &candidate_ids).await;
    let histories = history::get_pair_histories_batch(&state.redis, user_id, &candidate_ids).await;

    // Find the best match
    let mut best_match: Option<(f64, algorithm::QueueUser)> = None;
    let min_score = phase_a.min_match_score();

    for (i, candidate) in candidates.iter().enumerate() {
        // Check cooldown (from batch result)
        if cooldowns[i] {
            continue;
        }

        // Determine candidate's phase
        let wait_ms_b = (now_ms - candidate.joined_at).max(0);
        let phase_b = algorithm::MatchPhase::from_wait_ms(wait_ms_b);

        // Use batch-fetched pair history
        let pair_history = &histories[i];

        let score = algorithm::calculate_score(&current_user, candidate, phase_a, phase_b, pair_history);
        if score.passes_filters && score.score >= min_score {
            match &best_match {
                None => best_match = Some((score.score, (*candidate).clone())),
                Some((best_score, _)) if score.score > *best_score => {
                    best_match = Some((score.score, (*candidate).clone()));
                }
                _ => {}
            }
        }
    }

    if let Some((_score, partner)) = best_match {
        // Remove both from queue - verify they're actually still there
        let removed_self = queue::remove_from_queue(&state.redis, user_id).await.unwrap_or(false);
        let removed_partner = queue::remove_from_queue(&state.redis, &partner.user_id).await.unwrap_or(false);

        if !removed_self || !removed_partner {
            // One of them was already matched by a concurrent operation, put back
            if removed_self {
                let _ = queue::add_to_queue(&state.redis, &current_user).await;
            }
            if removed_partner {
                let _ = queue::add_to_queue(&state.redis, &partner).await;
            }
            tracing::warn!("race condition detected in try_match, retrying");
            let _ = socket.emit("searching", &serde_json::json!({ "status": "searching" }));
            return;
        }

        // Create match session in DB
        let db = state.db.clone();
        let new_session = NewMatchSession {
            user_a_id: *user_id,
            user_b_id: partner.user_id,
        };

        let match_session = {
            let mut conn = match db.get() {
                Ok(c) => c,
                Err(e) => {
                    tracing::error!(error = %e, "failed to get db connection");
                    // Put users back in queue
                    let _ = queue::add_to_queue(&state.redis, &current_user).await;
                    let _ = queue::add_to_queue(&state.redis, &partner).await;
                    return;
                }
            };

            match diesel::insert_into(match_sessions::table)
                .values(&new_session)
                .get_result::<crate::models::MatchSession>(&mut conn)
            {
                Ok(session) => session,
                Err(e) => {
                    tracing::error!(error = %e, "failed to create match session");
                    let _ = queue::add_to_queue(&state.redis, &current_user).await;
                    let _ = queue::add_to_queue(&state.redis, &partner).await;
                    return;
                }
            }
        };

        let match_id = match_session.id;

        // Set active pair in Redis
        queue::set_active_pair(&state.redis, &match_id, user_id, &partner.user_id).await;

        // Set cooldown so they do not get matched again immediately
        queue::set_cooldown(&state.redis, user_id, &partner.user_id).await;

        // Emit match-found to the current user
        let payload_for_current = MatchFoundPayload {
            match_id,
            partner: PartnerInfo {
                user_id: partner.user_id,
                display_name: partner.display_name.clone(),
                bio: partner.bio.clone(),
                age: partner.age,
                country: partner.country.clone(),
                kinks: partner.kinks.clone(),
                profile_photo_url: partner.profile_photo_url.clone(),
            },
            is_initiator: true,
        };
        let _ = socket.emit("match-found", &payload_for_current);

        // Emit match-found to the partner
        let payload_for_partner = MatchFoundPayload {
            match_id,
            partner: PartnerInfo {
                user_id: current_user.user_id,
                display_name: current_user.display_name.clone(),
                bio: current_user.bio.clone(),
                age: current_user.age,
                country: current_user.country.clone(),
                kinks: current_user.kinks.clone(),
                profile_photo_url: current_user.profile_photo_url.clone(),
            },
            is_initiator: false,
        };
        let partner_room = format!("user:{}", partner.user_id);
        let _ = socket.to(partner_room).emit("match-found", &payload_for_partner);

        // Publish event
        publisher::publish_session_started(
            &state.rabbitmq,
            match_id,
            *user_id,
            partner.user_id,
        )
        .await;

        tracing::info!(
            match_id = %match_id,
            user_a = %user_id,
            user_b = %partner.user_id,
            "match created"
        );
    } else {
        // No match found, user stays in queue
        let _ = socket.emit("searching", &serde_json::json!({ "status": "searching" }));
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn authenticate_socket(socket: &SocketRef, state: &Arc<AppState>) -> Result<Uuid, String> {
    let connect_info = socket.req_parts();

    // Extract token from query string
    let query = connect_info.uri.query().unwrap_or_default();
    let token = query
        .split('&')
        .find_map(|pair| {
            let mut split = pair.splitn(2, '=');
            let key = split.next()?;
            let value = split.next()?;
            if key == "token" {
                Some(value.to_string())
            } else {
                None
            }
        })
        .ok_or_else(|| "missing token query parameter".to_string())?;

    // Validate JWT
    let mut validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::HS256);
    validation.validate_exp = true;

    let token_data = jsonwebtoken::decode::<broz_shared::types::auth::Claims>(
        &token,
        &jsonwebtoken::DecodingKey::from_secret(state.config.jwt_secret.as_bytes()),
        &validation,
    )
    .map_err(|e| format!("invalid token: {e}"))?;

    if token_data.claims.is_expired() {
        return Err("token has expired".into());
    }

    // Store token in extensions for later use (e.g., forwarding to user service)
    socket.extensions.insert(token);

    Ok(token_data.claims.sub)
}

fn get_user_id(socket: &SocketRef) -> Option<Uuid> {
    socket.extensions.get::<Uuid>()
}

fn get_user_token(socket: &SocketRef) -> String {
    socket
        .extensions
        .get::<String>()
        .map(|t| t.clone())
        .unwrap_or_default()
}

async fn end_match_session(
    state: &Arc<AppState>,
    socket: &SocketRef,
    match_id: &Uuid,
    user_id: &Uuid,
    reason: &str,
) {
    // Get partner before removing pair
    let partner_id = queue::get_partner(&state.redis, match_id, user_id).await;

    // Calculate duration
    let duration_secs = {
        let db = state.db.clone();
        let match_id = *match_id;
        let reason = reason.to_string();
        let mut conn = match db.get() {
            Ok(c) => c,
            Err(e) => {
                tracing::error!(error = %e, "failed to get db connection for ending match");
                queue::remove_active_pair(&state.redis, &match_id).await;
                return;
            }
        };

        // Get the match session to calculate duration
        let session = match match_sessions::table
            .find(match_id)
            .first::<crate::models::MatchSession>(&mut conn)
        {
            Ok(s) => s,
            Err(e) => {
                tracing::error!(error = %e, match_id = %match_id, "match session not found");
                // Still clean up Redis state
                queue::remove_active_pair(&state.redis, &match_id).await;
                return;
            }
        };

        let now = Utc::now();
        let duration = (now - session.started_at).num_seconds() as i32;

        // Update the match session
        let _ = diesel::update(match_sessions::table.find(match_id))
            .set((
                match_sessions::ended_at.eq(Some(now)),
                match_sessions::end_reason.eq(Some(&reason)),
                match_sessions::duration_secs.eq(Some(duration)),
            ))
            .execute(&mut conn);

        duration
    };

    // Record pair history before cleaning up
    if let Some(pid) = partner_id {
        history::record_match_end(
            &state.redis,
            user_id,
            &pid,
            duration_secs.max(0) as u32,
            match_id,
        )
        .await;
    }

    // Remove active pair from Redis
    queue::remove_active_pair(&state.redis, match_id).await;

    // Notify partner
    if let Some(pid) = partner_id {
        let partner_room = format!("user:{pid}");
        let _ = socket.to(partner_room).emit(
            "partner-left",
            &serde_json::json!({
                "match_id": match_id,
                "reason": reason,
            }),
        );

        // Publish event
        publisher::publish_session_ended(
            &state.rabbitmq,
            *match_id,
            *user_id,
            pid,
            duration_secs,
            reason,
        )
        .await;
    }
}
