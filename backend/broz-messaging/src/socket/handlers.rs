use std::sync::Arc;

use serde::{Deserialize, Serialize};
use socketioxide::extract::{Data, SocketRef};
use uuid::Uuid;

use crate::AppState;

#[derive(Debug, Clone)]
pub struct CallSession {
    pub id: Uuid,
    pub room_id: String,
    pub caller_id: Uuid,
    pub callee_id: Uuid,
    pub caller_token: String,
    pub callee_token: String,
}

#[derive(Debug, Serialize)]
pub struct ErrorPayload {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Deserialize)]
struct CreateRoomTokens {
    caller: String,
    callee: String,
}

#[derive(Debug, Deserialize)]
struct CreateRoomResponse {
    id: String,
    tokens: CreateRoomTokens,
}

fn get_user_id(socket: &SocketRef) -> Option<Uuid> {
    socket.extensions.get::<Uuid>()
}

pub async fn on_connect_with_state(socket: SocketRef, state: Arc<AppState>) {
    let user_id = match authenticate_socket(&socket, &state) {
        Ok(id) => id,
        Err(msg) => {
            tracing::warn!(error = %msg, "messaging socket auth failed");
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

    // Join user-specific room so we can push messages to this user
    let user_room = format!("user:{user_id}");
    socket.join(user_room).ok();

    tracing::info!(user_id = %user_id, sid = %socket.id, "messaging socket connected");

    // Set presence in Redis
    let _ = state.redis.set(&format!("online:msg:{user_id}"), "1", 120).await;
    let _ = state.redis.set(&format!("online:{user_id}"), "1", 120).await;

    // Update DB presence via broz-user (fire-and-forget)
    let state_presence = state.clone();
    let uid = user_id;
    tokio::spawn(async move {
        update_presence(&state_presence, uid, true).await;
        notify_followers_presence(&state_presence, uid, true).await;
    });

    let _ = socket.emit("connected", &serde_json::json!({ "user_id": user_id }));

    // Register call event handlers
    socket.on("call-invite", {
        let state = state.clone();
        move |socket: SocketRef, Data::<serde_json::Value>(payload)| {
            let state = state.clone();
            async move { on_call_invite(socket, payload, &state).await; }
        }
    });

    socket.on("call-accept", {
        let state = state.clone();
        move |socket: SocketRef, Data::<serde_json::Value>(payload)| {
            let state = state.clone();
            async move { on_call_accept(socket, payload, &state).await; }
        }
    });

    socket.on("call-decline", {
        let state = state.clone();
        move |socket: SocketRef, Data::<serde_json::Value>(payload)| {
            let state = state.clone();
            async move { on_call_decline(socket, payload, &state).await; }
        }
    });

    socket.on("call-end", {
        let state = state.clone();
        move |socket: SocketRef, Data::<serde_json::Value>(payload)| {
            let state = state.clone();
            async move { on_call_end(socket, payload, &state).await; }
        }
    });

    // Heartbeat handler - refresh presence TTL
    socket.on("heartbeat", {
        let state = state.clone();
        move |socket: SocketRef| {
            let state = state.clone();
            async move {
                if let Some(user_id) = get_user_id(&socket) {
                    let _ = state.redis.set(&format!("online:msg:{user_id}"), "1", 120).await;
                    let _ = state.redis.set(&format!("online:{user_id}"), "1", 120).await;
                }
            }
        }
    });

    // Disconnect handler with state for presence cleanup
    socket.on_disconnect({
        let state = state.clone();
        move |socket: SocketRef| {
            let state = state.clone();
            async move {
                on_disconnect_with_state(socket, state).await;
            }
        }
    });
}

async fn on_disconnect_with_state(socket: SocketRef, state: Arc<AppState>) {
    let user_id = match get_user_id(&socket) {
        Some(id) => id,
        None => return,
    };

    tracing::info!(user_id = %user_id, sid = %socket.id, "messaging socket disconnected");

    // Remove messaging presence key
    let _ = state.redis.del(&format!("online:msg:{user_id}")).await;

    // Check if matching service is still connected
    let match_still_online = state.redis.exists(&format!("online:match:{user_id}")).await.unwrap_or(false);

    if !match_still_online {
        // User is fully offline - remove main presence key
        let _ = state.redis.del(&format!("online:{user_id}")).await;

        // Update DB + notify followers (fire-and-forget)
        let state_offline = state.clone();
        tokio::spawn(async move {
            update_presence(&state_offline, user_id, false).await;
            notify_followers_presence(&state_offline, user_id, false).await;
        });
    }

    // Auto-decline any pending calls for this user
    let calls_to_decline: Vec<Uuid> = state.active_calls
        .iter()
        .filter(|entry| entry.value().callee_id == user_id || entry.value().caller_id == user_id)
        .map(|entry| *entry.key())
        .collect();

    for call_id in calls_to_decline {
        if let Some((_, session)) = state.active_calls.remove(&call_id) {
            let partner_id = if session.caller_id == user_id {
                session.callee_id
            } else {
                session.caller_id
            };
            let partner_room = format!("user:{partner_id}");
            let _ = socket.to(partner_room).emit(
                "call-ended",
                &serde_json::json!({ "call_id": call_id }),
            );
            delete_sfu_room(&state, &session.room_id).await;
        }
    }
}

async fn on_call_invite(socket: SocketRef, payload: serde_json::Value, state: &Arc<AppState>) {
    let user_id = match get_user_id(&socket) {
        Some(id) => id,
        None => return,
    };

    let to = match payload.get("to").and_then(|v| v.as_str()).and_then(|s| Uuid::parse_str(s).ok()) {
        Some(id) => id,
        None => {
            tracing::warn!("call-invite missing 'to' field");
            return;
        }
    };

    let caller_name = payload.get("caller_name").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let caller_photo = payload.get("caller_photo").and_then(|v| v.as_str()).map(|s| s.to_string());

    // Create a room on LiveRelay SFU
    let api_url = &state.config.liverelay_api_url;
    let api_key = &state.config.liverelay_api_key;

    let room_res = state.http_client
        .post(format!("{api_url}/v1/rooms"))
        .header("Authorization", format!("Bearer {api_key}"))
        .json(&serde_json::json!({ "room_type": "call" }))
        .send()
        .await;

    let room = match room_res {
        Ok(res) if res.status().is_success() => {
            match res.json::<CreateRoomResponse>().await {
                Ok(r) => r,
                Err(e) => {
                    tracing::error!(error = %e, "failed to parse LiveRelay room response");
                    let _ = socket.emit("error", &ErrorPayload {
                        code: "SFU_ERROR".into(),
                        message: "Failed to create video room".into(),
                    });
                    return;
                }
            }
        }
        Ok(res) => {
            tracing::error!(status = %res.status(), "LiveRelay room creation failed");
            let _ = socket.emit("error", &ErrorPayload {
                code: "SFU_ERROR".into(),
                message: "Failed to create video room".into(),
            });
            return;
        }
        Err(e) => {
            tracing::error!(error = %e, "LiveRelay request failed");
            let _ = socket.emit("error", &ErrorPayload {
                code: "SFU_ERROR".into(),
                message: "Failed to reach video server".into(),
            });
            return;
        }
    };

    let call_id = Uuid::new_v4();
    let session = CallSession {
        id: call_id,
        room_id: room.id.clone(),
        caller_id: user_id,
        callee_id: to,
        caller_token: room.tokens.caller.clone(),
        callee_token: room.tokens.callee.clone(),
    };
    state.active_calls.insert(call_id, session);

    // Send call-created to caller with SFU token
    let _ = socket.emit(
        "call-created",
        &serde_json::json!({
            "call_id": call_id,
            "room_id": room.id,
            "sfu_token": room.tokens.caller,
        }),
    );

    // Send incoming-call to callee with SFU token
    let partner_room = format!("user:{to}");
    let _ = socket.to(partner_room).emit(
        "incoming-call",
        &serde_json::json!({
            "call_id": call_id,
            "room_id": room.id,
            "sfu_token": room.tokens.callee,
            "caller_id": user_id,
            "caller_name": caller_name,
            "caller_photo": caller_photo,
        }),
    );

    tracing::info!(call_id = %call_id, room_id = %room.id, caller = %user_id, callee = %to, "call invite sent via SFU");
}

async fn on_call_accept(socket: SocketRef, payload: serde_json::Value, state: &Arc<AppState>) {
    let user_id = match get_user_id(&socket) {
        Some(id) => id,
        None => return,
    };

    let call_id = match payload.get("call_id").and_then(|v| v.as_str()).and_then(|s| Uuid::parse_str(s).ok()) {
        Some(id) => id,
        None => {
            tracing::warn!("call-accept missing call_id");
            return;
        }
    };

    let session = match state.active_calls.get(&call_id) {
        Some(s) => s.clone(),
        None => {
            tracing::warn!(call_id = %call_id, "call-accept: call not found");
            return;
        }
    };

    // Notify caller that call was accepted
    let caller_room = format!("user:{}", session.caller_id);
    let _ = socket.to(caller_room).emit(
        "call-accepted",
        &serde_json::json!({ "call_id": call_id }),
    );
    // Notify callee (the sender) too
    let _ = socket.emit(
        "call-accepted",
        &serde_json::json!({ "call_id": call_id }),
    );

    tracing::info!(call_id = %call_id, accepted_by = %user_id, "call accepted");
}

async fn on_call_decline(socket: SocketRef, payload: serde_json::Value, state: &Arc<AppState>) {
    let user_id = match get_user_id(&socket) {
        Some(id) => id,
        None => return,
    };

    let call_id = match payload.get("call_id").and_then(|v| v.as_str()).and_then(|s| Uuid::parse_str(s).ok()) {
        Some(id) => id,
        None => {
            tracing::warn!("call-decline missing call_id");
            return;
        }
    };

    if let Some((_, session)) = state.active_calls.remove(&call_id) {
        // Notify the caller that the call was declined
        let caller_room = format!("user:{}", session.caller_id);
        let _ = socket.to(caller_room).emit(
            "call-declined",
            &serde_json::json!({ "call_id": call_id }),
        );

        // Delete room on LiveRelay SFU (fire-and-forget)
        delete_sfu_room(state, &session.room_id).await;

        tracing::info!(call_id = %call_id, declined_by = %user_id, "call declined");
    }
}

async fn on_call_end(socket: SocketRef, payload: serde_json::Value, state: &Arc<AppState>) {
    let user_id = match get_user_id(&socket) {
        Some(id) => id,
        None => return,
    };

    let call_id = match payload.get("call_id").and_then(|v| v.as_str()).and_then(|s| Uuid::parse_str(s).ok()) {
        Some(id) => id,
        None => {
            tracing::warn!("call-end missing call_id");
            return;
        }
    };

    if let Some((_, session)) = state.active_calls.remove(&call_id) {
        // Determine partner: if sender is caller, partner is callee and vice versa
        let partner_id = if user_id == session.caller_id {
            session.callee_id
        } else {
            session.caller_id
        };

        let partner_room = format!("user:{partner_id}");
        let _ = socket.to(partner_room).emit(
            "call-ended",
            &serde_json::json!({ "call_id": call_id }),
        );

        // Delete room on LiveRelay SFU (fire-and-forget)
        delete_sfu_room(state, &session.room_id).await;

        tracing::info!(call_id = %call_id, ended_by = %user_id, "call ended");
    }
}

async fn delete_sfu_room(state: &Arc<AppState>, room_id: &str) {
    let api_url = &state.config.liverelay_api_url;
    let api_key = &state.config.liverelay_api_key;

    if let Err(e) = state.http_client
        .delete(format!("{api_url}/v1/rooms/{room_id}"))
        .header("Authorization", format!("Bearer {api_key}"))
        .send()
        .await
    {
        tracing::warn!(error = %e, room_id = %room_id, "failed to delete SFU room");
    }
}

// ---------------------------------------------------------------------------
// Presence helpers
// ---------------------------------------------------------------------------

/// Update is_online + last_seen_at in broz-user DB via internal endpoint
async fn update_presence(state: &Arc<AppState>, user_id: Uuid, is_online: bool) {
    let url = format!("{}/internal/presence", state.config.user_service_url);
    if let Err(e) = state.http_client
        .post(&url)
        .json(&serde_json::json!({
            "user_id": user_id,
            "is_online": is_online,
        }))
        .send()
        .await
    {
        tracing::warn!(error = %e, user_id = %user_id, "failed to update presence in broz-user");
    }
}

/// Notify followers of presence change via Socket.IO
async fn notify_followers_presence(state: &Arc<AppState>, user_id: Uuid, is_online: bool) {
    // Get follower credential_ids from broz-user
    let url = format!("{}/internal/follower-ids/{}", state.config.user_service_url, user_id);
    let res = match state.http_client.get(&url).send().await {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(error = %e, "failed to fetch follower ids");
            return;
        }
    };

    #[derive(Deserialize)]
    struct FollowerIdsResponse {
        follower_ids: Vec<Uuid>,
    }

    let follower_ids = match res.json::<FollowerIdsResponse>().await {
        Ok(data) => data.follower_ids,
        Err(e) => {
            tracing::warn!(error = %e, "failed to parse follower ids response");
            return;
        }
    };

    if follower_ids.is_empty() {
        return;
    }

    let event = if is_online { "user-online" } else { "user-offline" };
    let payload = serde_json::json!({ "user_id": user_id });

    for follower_id in follower_ids {
        let room = format!("user:{follower_id}");
        let _ = state.io.to(room).emit(event, &payload);
    }

    tracing::debug!(user_id = %user_id, event = event, "presence notification sent to followers");
}

fn authenticate_socket(socket: &SocketRef, state: &Arc<AppState>) -> Result<Uuid, String> {
    let connect_info = socket.req_parts();

    // Extract token from query string ?token=xxx
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

    Ok(token_data.claims.sub)
}
