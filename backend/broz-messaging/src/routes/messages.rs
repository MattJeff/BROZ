use axum::extract::{Multipart, Path, Query, State};
use axum::Json;
use chrono::Utc;
use diesel::prelude::*;
use diesel::dsl::count_star;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use broz_shared::errors::{AppError, AppResult, ErrorCode};
use broz_shared::types::api::ApiResponse;
use broz_shared::types::auth::AuthUser;
use broz_shared::types::pagination::{PaginationParams, Paginated};

use crate::AppState;
use crate::events::publisher;
use crate::models::{Conversation, Message, NewConversation, NewConversationMember, NewMessage};
use crate::schema::{conversation_members, conversations, messages};

// --- Helper: build enriched socket payload with is_group info ---
fn build_socket_payload(
    conn: &mut diesel::pg::PgConnection,
    conversation_id: Uuid,
    message: &Message,
) -> serde_json::Value {
    let conv_info: Option<Conversation> = conversations::table
        .find(conversation_id)
        .first::<Conversation>(conn)
        .optional()
        .unwrap_or(None);

    serde_json::json!({
        "conversation_id": conversation_id,
        "is_group": conv_info.as_ref().map_or(false, |c| c.is_group),
        "group_name": conv_info.as_ref().and_then(|c| c.group_name.clone()),
        "message": {
            "id": message.id,
            "conversation_id": message.conversation_id,
            "sender_id": message.sender_id,
            "content": message.content,
            "media_url": message.media_url,
            "media_type": message.media_type,
            "is_private": message.is_private,
            "created_at": message.created_at,
        }
    })
}

// --- Request DTOs ---

#[derive(Debug, Deserialize)]
pub struct SendMessageRequest {
    pub content: Option<String>,
    pub media_url: Option<String>,
    pub media_type: Option<String>,
}

// --- Response DTOs ---

#[derive(Debug, Serialize)]
pub struct UnreadCountResponse {
    pub total_unread: i64,
}

// --- Helpers ---

/// Verify the user is a member of the given conversation. Returns an error if not.
fn verify_membership(
    conn: &mut diesel::pg::PgConnection,
    conversation_id: Uuid,
    user_id: Uuid,
) -> AppResult<()> {
    let is_member: bool = conversation_members::table
        .filter(conversation_members::conversation_id.eq(conversation_id))
        .filter(conversation_members::user_id.eq(user_id))
        .select(count_star())
        .first::<i64>(conn)
        .map(|c| c > 0)
        .map_err(|e| AppError::Database(e))?;

    if !is_member {
        return Err(AppError::new(
            ErrorCode::NotConversationMember,
            "you are not a member of this conversation",
        ));
    }

    Ok(())
}

// --- Handlers ---

/// GET /conversations/:id/messages - get paginated messages for a conversation
pub async fn list_messages(
    auth_user: AuthUser,
    State(state): State<Arc<AppState>>,
    Path(conversation_id): Path<Uuid>,
    Query(params): Query<PaginationParams>,
) -> AppResult<Json<ApiResponse<Paginated<Message>>>> {
    let mut conn = state.db.get().map_err(|e| AppError::Internal(e.into()))?;

    verify_membership(&mut conn, conversation_id, auth_user.id)?;

    // Get total count
    let total: i64 = messages::table
        .filter(messages::conversation_id.eq(conversation_id))
        .select(count_star())
        .first::<i64>(&mut conn)
        .map_err(|e| AppError::Database(e))?;

    // Get paginated messages ordered by created_at descending (newest first)
    let items: Vec<Message> = messages::table
        .filter(messages::conversation_id.eq(conversation_id))
        .order(messages::created_at.desc())
        .offset(params.offset() as i64)
        .limit(params.limit() as i64)
        .load::<Message>(&mut conn)
        .map_err(|e| AppError::Database(e))?;

    let paginated = Paginated::new(items, total as u64, &params);

    Ok(Json(ApiResponse::ok(paginated)))
}

/// POST /conversations/:id/messages - send a message in a conversation
pub async fn send_message(
    auth_user: AuthUser,
    State(state): State<Arc<AppState>>,
    Path(conversation_id): Path<Uuid>,
    Json(req): Json<SendMessageRequest>,
) -> AppResult<Json<ApiResponse<Message>>> {
    // Validate that at least content or media is provided
    if req.content.as_ref().map_or(true, |c| c.trim().is_empty())
        && req.media_url.as_ref().map_or(true, |u| u.trim().is_empty())
    {
        return Err(AppError::new(
            ErrorCode::ValidationError,
            "message must have content or media",
        ));
    }

    let mut conn = state.db.get().map_err(|e| AppError::Internal(e.into()))?;

    verify_membership(&mut conn, conversation_id, auth_user.id)?;

    let new_message = NewMessage {
        conversation_id,
        sender_id: auth_user.id,
        content: req.content.clone(),
        media_url: req.media_url,
        media_type: req.media_type,
        is_private: false,
    };

    let message: Message = diesel::insert_into(messages::table)
        .values(&new_message)
        .get_result(&mut conn)
        .map_err(|e| AppError::Database(e))?;

    // Build a content preview for the event (truncated to 100 chars)
    let content_preview = req
        .content
        .as_deref()
        .unwrap_or("[media]")
        .chars()
        .take(100)
        .collect::<String>();

    // Publish the message.sent event
    publisher::publish_message_sent(
        &state.rabbitmq,
        message.id,
        conversation_id,
        auth_user.id,
        &auth_user.id.to_string(), // sender display name not available here; using ID as fallback
        &content_preview,
    )
    .await;

    // Emit new_message via Socket.IO to all conversation members
    let member_ids: Vec<Uuid> = conversation_members::table
        .filter(conversation_members::conversation_id.eq(conversation_id))
        .select(conversation_members::user_id)
        .load::<Uuid>(&mut conn)
        .unwrap_or_default();

    let socket_payload = build_socket_payload(&mut conn, conversation_id, &message);

    tracing::info!(
        sender = %auth_user.id,
        conversation = %conversation_id,
        members = ?member_ids,
        "emitting new_message socket event (send_message)"
    );

    for member_id in &member_ids {
        if *member_id == auth_user.id {
            continue; // Don't send to the sender
        }
        let room = format!("user:{member_id}");
        let result = state.io.to(room.clone()).emit("new_message", &socket_payload);
        tracing::info!(
            target_user = %member_id,
            room = %room,
            success = result.is_ok(),
            "socket emit new_message (send_message)"
        );
    }

    Ok(Json(ApiResponse::ok(message)))
}

/// DELETE /messages/:id - soft delete a message (only the sender can delete)
pub async fn delete_message(
    auth_user: AuthUser,
    State(state): State<Arc<AppState>>,
    Path(message_id): Path<Uuid>,
) -> AppResult<Json<ApiResponse<Message>>> {
    let mut conn = state.db.get().map_err(|e| AppError::Internal(e.into()))?;

    // Find the message
    let message: Message = messages::table
        .find(message_id)
        .first::<Message>(&mut conn)
        .optional()
        .map_err(|e| AppError::Database(e))?
        .ok_or_else(|| AppError::new(ErrorCode::MessageNotFound, "message not found"))?;

    // Only the sender can delete their own message
    if message.sender_id != auth_user.id {
        return Err(AppError::new(
            ErrorCode::Forbidden,
            "you can only delete your own messages",
        ));
    }

    // Soft delete: set is_deleted = true
    let updated: Message = diesel::update(messages::table.find(message_id))
        .set(messages::is_deleted.eq(true))
        .get_result(&mut conn)
        .map_err(|e| AppError::Database(e))?;

    Ok(Json(ApiResponse::ok(updated)))
}

/// POST /conversations/:id/read - mark conversation as read (update last_read_at)
pub async fn mark_as_read(
    auth_user: AuthUser,
    State(state): State<Arc<AppState>>,
    Path(conversation_id): Path<Uuid>,
) -> AppResult<Json<ApiResponse<serde_json::Value>>> {
    let mut conn = state.db.get().map_err(|e| AppError::Internal(e.into()))?;

    let updated_rows = diesel::update(
        conversation_members::table
            .filter(conversation_members::conversation_id.eq(conversation_id))
            .filter(conversation_members::user_id.eq(auth_user.id)),
    )
    .set(conversation_members::last_read_at.eq(Utc::now()))
    .execute(&mut conn)
    .map_err(|e| AppError::Database(e))?;

    if updated_rows == 0 {
        return Err(AppError::new(
            ErrorCode::NotConversationMember,
            "you are not a member of this conversation",
        ));
    }

    Ok(Json(ApiResponse::ok(serde_json::json!({
        "conversation_id": conversation_id,
        "read_at": Utc::now()
    }))))
}

/// GET /unread-count - get total unread messages count across all conversations
pub async fn get_unread_count(
    auth_user: AuthUser,
    State(state): State<Arc<AppState>>,
) -> AppResult<Json<ApiResponse<UnreadCountResponse>>> {
    let mut conn = state.db.get().map_err(|e| AppError::Internal(e.into()))?;
    let user_id = auth_user.id;

    // Get all memberships with their last_read_at
    let memberships: Vec<(Uuid, chrono::DateTime<Utc>)> = conversation_members::table
        .filter(conversation_members::user_id.eq(user_id))
        .select((conversation_members::conversation_id, conversation_members::last_read_at))
        .load::<(Uuid, chrono::DateTime<Utc>)>(&mut conn)
        .map_err(|e| AppError::Database(e))?;

    let mut total_unread: i64 = 0;

    for (conv_id, last_read_at) in &memberships {
        let unread: i64 = messages::table
            .filter(messages::conversation_id.eq(conv_id))
            .filter(messages::created_at.gt(last_read_at))
            .filter(messages::sender_id.ne(user_id))
            .select(count_star())
            .first::<i64>(&mut conn)
            .map_err(|e| AppError::Database(e))?;

        total_unread += unread;
    }

    Ok(Json(ApiResponse::ok(UnreadCountResponse { total_unread })))
}

// --- POST /send ---

#[derive(Debug, Deserialize)]
pub struct SimpleSendRequest {
    pub conversation_id: Option<Uuid>,
    pub partner_id: Option<Uuid>,
    pub content: Option<String>,
    pub media_url: Option<String>,
    pub media_type: Option<String>,
    pub is_group: Option<bool>,
    pub participants: Option<Vec<Uuid>>,
}

#[derive(Debug, Serialize)]
pub struct SimpleSendResponse {
    pub conversation_id: Uuid,
    pub message: Message,
}

/// POST /send - send a message, auto-creating conversation if needed
pub async fn send_message_simple(
    auth_user: AuthUser,
    State(state): State<Arc<AppState>>,
    Json(req): Json<SimpleSendRequest>,
) -> AppResult<Json<ApiResponse<SimpleSendResponse>>> {
    if req.content.as_ref().map_or(true, |c| c.trim().is_empty())
        && req.media_url.as_ref().map_or(true, |u| u.trim().is_empty())
    {
        return Err(AppError::new(
            ErrorCode::ValidationError,
            "message must have content or media",
        ));
    }

    let mut conn = state.db.get().map_err(|e| AppError::Internal(e.into()))?;

    // Resolve or create conversation
    let conversation_id = if let Some(cid) = req.conversation_id {
        // Verify membership
        verify_membership(&mut conn, cid, auth_user.id)?;
        cid
    } else if req.is_group == Some(true) {
        // Create a new group conversation with participants
        let participants = req.participants.as_deref().unwrap_or(&[]);
        if participants.is_empty() {
            return Err(AppError::new(
                ErrorCode::ValidationError,
                "participants required for group conversation",
            ));
        }
        create_group_conversation(&mut conn, auth_user.id, participants)?
    } else if let Some(partner_id) = req.partner_id {
        // Find existing 1-on-1 conversation or create one
        find_or_create_dm(&mut conn, auth_user.id, partner_id)?
    } else {
        return Err(AppError::new(
            ErrorCode::ValidationError,
            "conversation_id or partner_id is required",
        ));
    };

    // Insert the message
    let new_message = NewMessage {
        conversation_id,
        sender_id: auth_user.id,
        content: req.content.clone(),
        media_url: req.media_url,
        media_type: req.media_type,
        is_private: false,
    };

    let message: Message = diesel::insert_into(messages::table)
        .values(&new_message)
        .get_result(&mut conn)
        .map_err(|e| AppError::Database(e))?;

    // Publish RabbitMQ event
    let content_preview = req
        .content
        .as_deref()
        .unwrap_or("[media]")
        .chars()
        .take(100)
        .collect::<String>();

    publisher::publish_message_sent(
        &state.rabbitmq,
        message.id,
        conversation_id,
        auth_user.id,
        &auth_user.id.to_string(),
        &content_preview,
    )
    .await;

    // Emit new_message via Socket.IO to all conversation members
    let member_ids: Vec<Uuid> = conversation_members::table
        .filter(conversation_members::conversation_id.eq(conversation_id))
        .select(conversation_members::user_id)
        .load::<Uuid>(&mut conn)
        .unwrap_or_default();

    let socket_payload = build_socket_payload(&mut conn, conversation_id, &message);

    tracing::info!(
        sender = %auth_user.id,
        conversation = %conversation_id,
        members = ?member_ids,
        "emitting new_message socket event"
    );

    for member_id in &member_ids {
        if *member_id == auth_user.id {
            continue;
        }
        let room = format!("user:{member_id}");
        let result = state.io.to(room.clone()).emit("new_message", &socket_payload);
        tracing::info!(
            target_user = %member_id,
            room = %room,
            success = result.is_ok(),
            "socket emit new_message"
        );
    }

    Ok(Json(ApiResponse::ok(SimpleSendResponse {
        conversation_id,
        message,
    })))
}

/// Find an existing 1-on-1 conversation between two users, or create one.
fn find_or_create_dm(
    conn: &mut diesel::pg::PgConnection,
    user_a: Uuid,
    user_b: Uuid,
) -> AppResult<Uuid> {
    // Find conversation IDs where user_a is a member
    let a_convs: Vec<Uuid> = conversation_members::table
        .filter(conversation_members::user_id.eq(user_a))
        .select(conversation_members::conversation_id)
        .load::<Uuid>(conn)
        .map_err(|e| AppError::Database(e))?;

    if !a_convs.is_empty() {
        // Find a non-group conversation that also has user_b
        for conv_id in &a_convs {
            let conv: Option<Conversation> = conversations::table
                .find(conv_id)
                .filter(conversations::is_group.eq(false))
                .first::<Conversation>(conn)
                .optional()
                .map_err(|e| AppError::Database(e))?;

            if conv.is_some() {
                let has_b: bool = conversation_members::table
                    .filter(conversation_members::conversation_id.eq(conv_id))
                    .filter(conversation_members::user_id.eq(user_b))
                    .select(count_star())
                    .first::<i64>(conn)
                    .map(|c| c > 0)
                    .map_err(|e| AppError::Database(e))?;

                if has_b {
                    return Ok(*conv_id);
                }
            }
        }
    }

    // No existing DM, create one
    let new_conv = NewConversation {
        is_group: false,
        group_name: None,
        group_photo_url: None,
    };

    let conversation: Conversation = diesel::insert_into(conversations::table)
        .values(&new_conv)
        .get_result(conn)
        .map_err(|e| AppError::Database(e))?;

    let members = vec![
        NewConversationMember {
            conversation_id: conversation.id,
            user_id: user_a,
        },
        NewConversationMember {
            conversation_id: conversation.id,
            user_id: user_b,
        },
    ];

    diesel::insert_into(conversation_members::table)
        .values(&members)
        .execute(conn)
        .map_err(|e| AppError::Database(e))?;

    Ok(conversation.id)
}

/// Create a new group conversation with the given participants + the creator.
fn create_group_conversation(
    conn: &mut diesel::pg::PgConnection,
    creator_id: Uuid,
    participant_ids: &[Uuid],
) -> AppResult<Uuid> {
    let new_conv = NewConversation {
        is_group: true,
        group_name: None,
        group_photo_url: None,
    };

    let conversation: Conversation = diesel::insert_into(conversations::table)
        .values(&new_conv)
        .get_result(conn)
        .map_err(|e| AppError::Database(e))?;

    // Collect unique member IDs (creator + participants)
    let mut unique_ids = std::collections::HashSet::new();
    unique_ids.insert(creator_id);
    for pid in participant_ids {
        unique_ids.insert(*pid);
    }

    let members: Vec<NewConversationMember> = unique_ids
        .into_iter()
        .map(|uid| NewConversationMember {
            conversation_id: conversation.id,
            user_id: uid,
        })
        .collect();

    diesel::insert_into(conversation_members::table)
        .values(&members)
        .execute(conn)
        .map_err(|e| AppError::Database(e))?;

    Ok(conversation.id)
}

/// POST /send-media - upload a media file and send as a message
pub async fn send_media(
    auth_user: AuthUser,
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> AppResult<Json<ApiResponse<SimpleSendResponse>>> {
    let mut file_data: Option<(Vec<u8>, String)> = None; // (bytes, content_type)
    let mut partner_id: Option<Uuid> = None;
    let mut conversation_id: Option<Uuid> = None;
    let mut is_private = false;
    let mut is_group = false;
    let mut participants: Vec<Uuid> = Vec::new();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::new(ErrorCode::ValidationError, format!("multipart error: {e}")))?
    {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "file" => {
                let content_type = field
                    .content_type()
                    .unwrap_or("application/octet-stream")
                    .to_string();
                let data = field
                    .bytes()
                    .await
                    .map_err(|e| AppError::new(ErrorCode::ValidationError, format!("failed to read file: {e}")))?;
                file_data = Some((data.to_vec(), content_type));
            }
            "partner_id" => {
                let val = field.text().await.unwrap_or_default();
                partner_id = Uuid::parse_str(&val).ok();
            }
            "conversation_id" => {
                let val = field.text().await.unwrap_or_default();
                conversation_id = Uuid::parse_str(&val).ok();
            }
            "is_private" => {
                let val = field.text().await.unwrap_or_default();
                is_private = val == "true";
            }
            "is_group" => {
                let val = field.text().await.unwrap_or_default();
                is_group = val == "true";
            }
            "participants" => {
                let val = field.text().await.unwrap_or_default();
                if let Ok(ids) = serde_json::from_str::<Vec<Uuid>>(&val) {
                    participants = ids;
                }
            }
            _ => {}
        }
    }

    let (data, content_type) = file_data
        .ok_or_else(|| AppError::new(ErrorCode::ValidationError, "no file provided"))?;

    // Validate file type and determine extension
    let ext = match content_type.as_str() {
        "image/jpeg" | "image/jpg" => "jpg",
        "image/png" => "png",
        "image/webp" => "webp",
        "image/gif" => "gif",
        "video/mp4" => "mp4",
        "video/quicktime" => "mov",
        "video/webm" => "webm",
        _ => {
            return Err(AppError::new(
                ErrorCode::ValidationError,
                "unsupported format, accepted: jpeg, png, webp, gif, mp4, quicktime, webm",
            ));
        }
    };

    let mut conn = state.db.get().map_err(|e| AppError::Internal(e.into()))?;

    // Resolve or create conversation
    let conv_id = if let Some(cid) = conversation_id {
        verify_membership(&mut conn, cid, auth_user.id)?;
        cid
    } else if is_group && !participants.is_empty() {
        create_group_conversation(&mut conn, auth_user.id, &participants)?
    } else if let Some(pid) = partner_id {
        find_or_create_dm(&mut conn, auth_user.id, pid)?
    } else {
        return Err(AppError::new(
            ErrorCode::ValidationError,
            "conversation_id or partner_id is required",
        ));
    };

    // Upload to MinIO
    let file_id = Uuid::now_v7();
    let key = format!("messages/{}/{}.{}", conv_id, file_id, ext);

    let media_url = state
        .minio
        .upload(&key, data, &content_type)
        .await
        .map_err(|e| AppError::new(ErrorCode::ValidationError, e))?;

    // Insert message
    let new_message = NewMessage {
        conversation_id: conv_id,
        sender_id: auth_user.id,
        content: Some(if content_type.starts_with("image/") {
            "[Image]".to_string()
        } else {
            "[Média]".to_string()
        }),
        media_url: Some(media_url.clone()),
        media_type: Some(content_type.clone()),
        is_private,
    };

    let message: Message = diesel::insert_into(messages::table)
        .values(&new_message)
        .get_result(&mut conn)
        .map_err(|e| AppError::Database(e))?;

    // Publish RabbitMQ event
    let content_preview = if content_type.starts_with("image/") {
        "[image]"
    } else {
        "[media]"
    };
    publisher::publish_message_sent(
        &state.rabbitmq,
        message.id,
        conv_id,
        auth_user.id,
        &auth_user.id.to_string(),
        content_preview,
    )
    .await;

    // Emit new_message via Socket.IO to all conversation members
    let member_ids: Vec<Uuid> = conversation_members::table
        .filter(conversation_members::conversation_id.eq(conv_id))
        .select(conversation_members::user_id)
        .load::<Uuid>(&mut conn)
        .unwrap_or_default();

    let socket_payload = build_socket_payload(&mut conn, conv_id, &message);

    for member_id in &member_ids {
        if *member_id == auth_user.id {
            continue;
        }
        let room = format!("user:{member_id}");
        let _ = state.io.to(room).emit("new_message", &socket_payload);
    }

    tracing::info!(
        sender = %auth_user.id,
        conversation = %conv_id,
        media_type = %content_type,
        is_private = is_private,
        "media message sent"
    );

    Ok(Json(ApiResponse::ok(SimpleSendResponse {
        conversation_id: conv_id,
        message,
    })))
}
