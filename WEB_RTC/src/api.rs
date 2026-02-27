use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

// ---------------------------------------------------------------------------
// Request / Response DTOs
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct CreateRoomRequest {
    pub room_type: crate::room::RoomType,
}

#[derive(Serialize)]
pub struct CreateRoomResponse {
    pub id: String,
    #[serde(rename = "type")]
    pub room_type: crate::room::RoomType,
    pub tokens: RoomTokens,
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum RoomTokens {
    Broadcast { publish: String, subscribe: String },
    Call { caller: String, callee: String },
    /// Conference tokens: a list of N tokens (one per participant slot).
    Conference { tokens: Vec<String> },
}

#[derive(Deserialize)]
pub struct CreateTokenRequest {
    pub role: String,
}

#[derive(Serialize)]
pub struct CreateTokenResponse {
    pub token: String,
}

#[derive(Deserialize)]
pub struct CreateKeyRequest {
    pub name: String,
}

#[derive(Serialize)]
pub struct CreateKeyResponse {
    pub key: String,
    pub name: String,
}

// ---------------------------------------------------------------------------
// POST /v1/rooms — create a room
// ---------------------------------------------------------------------------

pub async fn create_room(
    State(state): State<Arc<crate::AppState>>,
    headers: HeaderMap,
    Json(body): Json<CreateRoomRequest>,
) -> Result<Json<CreateRoomResponse>, crate::error::ApiError> {
    let api_key = crate::auth::require_api_key(&headers, &state.api_keys)
        .await?;

    let room_id = uuid::Uuid::new_v4().to_string();

    let room = Arc::new(crate::room::Room::new(room_id.clone(), body.room_type));

    {
        let mut rooms = state.rooms.write().unwrap();
        rooms.insert(room_id.clone(), room);
    }

    info!("Room '{}' created (type={:?}) by key '{}'", room_id, body.room_type, api_key.name);

    // Emit room.created event.
    let room_type_str = match body.room_type {
        crate::room::RoomType::Broadcast => "broadcast",
        crate::room::RoomType::Call => "call",
        crate::room::RoomType::Conference => "conference",
    };
    state.event_bus.emit(crate::events::LiveRelayEvent::room_created(&room_id, room_type_str));

    const TTL: u64 = 86400; // 24 hours

    let tokens = match body.room_type {
        crate::room::RoomType::Broadcast => {
            let publish = crate::auth::create_token(
                &state.jwt_secret,
                &room_id,
                "publish",
                &api_key.key,
                TTL,
            )
            .map_err(|e| {
                tracing::warn!("Failed to create publish token: {e}");
                crate::error::ApiError::internal("Failed to create token")
            })?;

            let subscribe = crate::auth::create_token(
                &state.jwt_secret,
                &room_id,
                "subscribe",
                &api_key.key,
                TTL,
            )
            .map_err(|e| {
                tracing::warn!("Failed to create subscribe token: {e}");
                crate::error::ApiError::internal("Failed to create token")
            })?;

            RoomTokens::Broadcast { publish, subscribe }
        }
        crate::room::RoomType::Call => {
            let caller = crate::auth::create_token(
                &state.jwt_secret,
                &room_id,
                "call",
                &api_key.key,
                TTL,
            )
            .map_err(|e| {
                tracing::warn!("Failed to create call token: {e}");
                crate::error::ApiError::internal("Failed to create token")
            })?;

            let callee = crate::auth::create_token(
                &state.jwt_secret,
                &room_id,
                "call",
                &api_key.key,
                TTL,
            )
            .map_err(|e| {
                tracing::warn!("Failed to create call token: {e}");
                crate::error::ApiError::internal("Failed to create token")
            })?;

            RoomTokens::Call { caller, callee }
        }
        crate::room::RoomType::Conference => {
            // Generate a few initial tokens; more can be created via
            // POST /v1/rooms/:room_id/token with role "conference".
            let mut tokens = Vec::new();
            for _ in 0..4 {
                let token = crate::auth::create_token(
                    &state.jwt_secret,
                    &room_id,
                    "conference",
                    &api_key.key,
                    TTL,
                )
                .map_err(|e| {
                    tracing::warn!("Failed to create conference token: {e}");
                    crate::error::ApiError::internal("Failed to create token")
                })?;
                tokens.push(token);
            }

            RoomTokens::Conference { tokens }
        }
    };

    Ok(Json(CreateRoomResponse {
        id: room_id,
        room_type: body.room_type,
        tokens,
    }))
}

// ---------------------------------------------------------------------------
// GET /v1/rooms — list all rooms
// ---------------------------------------------------------------------------

pub async fn list_rooms(
    State(state): State<Arc<crate::AppState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<crate::room::RoomInfo>>, crate::error::ApiError> {
    crate::auth::require_api_key(&headers, &state.api_keys).await?;

    let rooms = state.rooms.read().unwrap();
    let infos: Vec<crate::room::RoomInfo> = rooms.values().map(|r| r.info()).collect();

    Ok(Json(infos))
}

// ---------------------------------------------------------------------------
// GET /v1/rooms/:room_id — get room info
// ---------------------------------------------------------------------------

pub async fn get_room(
    State(state): State<Arc<crate::AppState>>,
    Path(room_id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<crate::room::RoomInfo>, crate::error::ApiError> {
    crate::auth::require_api_key(&headers, &state.api_keys).await?;

    let rooms = state.rooms.read().unwrap();
    let room = rooms
        .get(&room_id)
        .ok_or_else(|| crate::error::ApiError::room_not_found(&room_id))?;

    Ok(Json(room.info()))
}

// ---------------------------------------------------------------------------
// DELETE /v1/rooms/:room_id — delete a room
// ---------------------------------------------------------------------------

pub async fn delete_room(
    State(state): State<Arc<crate::AppState>>,
    Path(room_id): Path<String>,
    headers: HeaderMap,
) -> Result<StatusCode, crate::error::ApiError> {
    crate::auth::require_api_key(&headers, &state.api_keys).await?;

    let room = {
        let mut rooms = state.rooms.write().unwrap();
        rooms.remove(&room_id)
    };

    match room {
        Some(room) => {
            // Close every publisher PeerConnection so media stops flowing.
            let publishers = room.get_publishers();
            for publisher in &publishers {
                if let Err(e) = publisher.pc.close().await {
                    tracing::warn!(
                        "Failed to close publisher '{}' PeerConnection in room '{}': {e}",
                        publisher.peer_id,
                        room_id,
                    );
                }
            }
            info!(
                "Room '{}' deleted ({} publisher(s) closed)",
                room_id,
                publishers.len()
            );

            // Emit room.deleted event.
            let room_type_str = match room.room_type {
                crate::room::RoomType::Broadcast => "broadcast",
                crate::room::RoomType::Call => "call",
                crate::room::RoomType::Conference => "conference",
            };
            state.event_bus.emit(crate::events::LiveRelayEvent::room_deleted(&room_id, room_type_str));

            Ok(StatusCode::NO_CONTENT)
        }
        None => Err(crate::error::ApiError::room_not_found(&room_id)),
    }
}

// ---------------------------------------------------------------------------
// POST /v1/rooms/:room_id/token — generate a token for a room
// ---------------------------------------------------------------------------

pub async fn create_room_token(
    State(state): State<Arc<crate::AppState>>,
    Path(room_id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<CreateTokenRequest>,
) -> Result<Json<CreateTokenResponse>, crate::error::ApiError> {
    let api_key = crate::auth::require_api_key(&headers, &state.api_keys)
        .await?;

    // Validate the requested role.
    if !crate::auth::validate_role(&body.role) {
        return Err(crate::error::ApiError::invalid_role(&body.role));
    }

    // Verify the room exists.
    {
        let rooms = state.rooms.read().unwrap();
        if !rooms.contains_key(&room_id) {
            return Err(crate::error::ApiError::room_not_found(&room_id));
        }
    }

    let token = crate::auth::create_token(
        &state.jwt_secret,
        &room_id,
        &body.role,
        &api_key.key,
        86400,
    )
    .map_err(|e| {
        tracing::warn!("Failed to create token for room '{}': {e}", room_id);
        crate::error::ApiError::internal("Failed to create token")
    })?;

    info!(
        "Token created for room '{}' role='{}' by key '{}'",
        room_id, body.role, api_key.name
    );

    Ok(Json(CreateTokenResponse { token }))
}

// ---------------------------------------------------------------------------
// POST /v1/keys — create an API key (requires authentication)
// ---------------------------------------------------------------------------

pub async fn create_api_key(
    State(state): State<Arc<crate::AppState>>,
    headers: HeaderMap,
    Json(body): Json<CreateKeyRequest>,
) -> Result<Json<CreateKeyResponse>, crate::error::ApiError> {
    crate::auth::require_api_key(&headers, &state.api_keys).await?;

    let key = crate::auth::generate_api_key();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_secs();

    let api_key = crate::auth::ApiKey {
        key: key.clone(),
        name: body.name.clone(),
        created_at: now,
    };

    {
        let mut keys = state.api_keys.write().unwrap();
        keys.insert(key.clone(), api_key);
    }

    info!("API key created: name='{}'", body.name);

    Ok(Json(CreateKeyResponse {
        key,
        name: body.name,
    }))
}
