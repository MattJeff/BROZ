use axum::extract::{Multipart, Path, State};
use axum::Json;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::dsl::count_star;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use broz_shared::errors::{AppError, AppResult, ErrorCode};
use broz_shared::types::api::ApiResponse;
use broz_shared::types::auth::AuthUser;

use crate::AppState;
use crate::models::{
    Conversation, ConversationMember, Message, NewConversation, NewConversationMember,
};
use crate::schema::{conversation_members, conversations, messages};

// --- Response DTOs ---

#[derive(Debug, Serialize)]
pub struct ConversationPreview {
    pub id: Uuid,
    pub is_group: bool,
    pub group_name: Option<String>,
    pub group_photo_url: Option<String>,
    pub partner_id: Option<Uuid>,
    pub partner_name: Option<String>,
    pub partner_photo: Option<String>,
    pub partner_country: Option<String>,
    pub partner_online: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_message: Option<String>,
    pub last_message_time: Option<DateTime<Utc>>,
    pub unread_count: i64,
}

#[derive(Debug, Serialize)]
pub struct ConversationDetail {
    #[serde(flatten)]
    pub conversation: Conversation,
    pub members: Vec<EnrichedMember>,
}

#[derive(Debug, Serialize, Clone)]
pub struct EnrichedMember {
    pub id: Uuid,
    pub user_id: Uuid,
    pub display_name: Option<String>,
    pub profile_photo: Option<String>,
    pub country: Option<String>,
    pub is_online: bool,
    pub joined_at: DateTime<Utc>,
}

// --- Request DTOs ---

#[derive(Debug, Deserialize)]
pub struct CreateGroupRequest {
    pub name: String,
    pub member_ids: Vec<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct AddMemberRequest {
    pub user_id: Uuid,
}

// --- Handlers ---

/// GET /conversations - list user's conversations with last message preview and unread count
pub async fn list_conversations(
    auth_user: AuthUser,
    State(state): State<Arc<AppState>>,
) -> AppResult<Json<ApiResponse<Vec<ConversationPreview>>>> {
    let mut conn = state.db.get().map_err(|e| AppError::Internal(e.into()))?;
    let user_id = auth_user.id;

    // Get all conversation IDs the user is a member of, along with last_read_at
    let memberships: Vec<(Uuid, DateTime<Utc>)> = conversation_members::table
        .filter(conversation_members::user_id.eq(user_id))
        .select((conversation_members::conversation_id, conversation_members::last_read_at))
        .order(conversation_members::joined_at.desc())
        .load::<(Uuid, DateTime<Utc>)>(&mut conn)
        .map_err(|e| AppError::Database(e))?;

    if memberships.is_empty() {
        return Ok(Json(ApiResponse::ok(vec![])));
    }

    let conv_ids: Vec<Uuid> = memberships.iter().map(|(id, _)| *id).collect();

    // Load conversations
    let convs: Vec<Conversation> = conversations::table
        .filter(conversations::id.eq_any(&conv_ids))
        .load::<Conversation>(&mut conn)
        .map_err(|e| AppError::Database(e))?;

    // Build previews
    let mut previews = Vec::with_capacity(convs.len());
    for conv in convs {
        let last_read_at = memberships
            .iter()
            .find(|(cid, _)| *cid == conv.id)
            .map(|(_, lr)| *lr)
            .unwrap_or(conv.created_at);

        // Get last message
        let last_msg: Option<Message> = messages::table
            .filter(messages::conversation_id.eq(conv.id))
            .order(messages::created_at.desc())
            .first::<Message>(&mut conn)
            .optional()
            .map_err(|e| AppError::Database(e))?;

        // Count unread messages
        let unread: i64 = messages::table
            .filter(messages::conversation_id.eq(conv.id))
            .filter(messages::created_at.gt(last_read_at))
            .filter(messages::sender_id.ne(user_id))
            .select(count_star())
            .first::<i64>(&mut conn)
            .map_err(|e| AppError::Database(e))?;

        // For DM conversations, find the partner_id (the other member)
        let partner_id = if !conv.is_group {
            conversation_members::table
                .filter(conversation_members::conversation_id.eq(conv.id))
                .filter(conversation_members::user_id.ne(user_id))
                .select(conversation_members::user_id)
                .first::<Uuid>(&mut conn)
                .optional()
                .map_err(|e| AppError::Database(e))?
        } else {
            None
        };

        let last_message_time = last_msg.as_ref().map(|m| m.created_at);
        let last_message_text = last_msg.map(|m| {
            if m.is_deleted {
                "Message supprimé".to_string()
            } else {
                m.content.unwrap_or_else(|| "[media]".to_string())
            }
        });

        previews.push(ConversationPreview {
            id: conv.id,
            is_group: conv.is_group,
            group_name: conv.group_name,
            group_photo_url: conv.group_photo_url,
            partner_id,
            partner_name: None,
            partner_photo: None,
            partner_country: None,
            partner_online: false,
            created_at: conv.created_at,
            updated_at: conv.updated_at,
            last_message: last_message_text,
            last_message_time,
            unread_count: unread,
        });
    }

    // Enrich DM previews with partner profile data from broz-user
    let partner_ids: Vec<Uuid> = previews.iter()
        .filter_map(|p| p.partner_id)
        .collect();
    if !partner_ids.is_empty() {
        let url = format!("{}/internal/profiles/batch", state.config.user_service_url);
        let client = reqwest::Client::new();
        if let Ok(resp) = client
            .post(&url)
            .json(&serde_json::json!({ "credential_ids": partner_ids }))
            .send()
            .await
        {
            if let Ok(profiles) = resp.json::<Vec<serde_json::Value>>().await {
                let profile_map: std::collections::HashMap<String, serde_json::Value> = profiles
                    .into_iter()
                    .filter_map(|p| {
                        p.get("credential_id")
                            .and_then(|v| v.as_str())
                            .map(|cid| (cid.to_string(), p.clone()))
                    })
                    .collect();

                for preview in &mut previews {
                    if let Some(pid) = preview.partner_id {
                        if let Some(profile) = profile_map.get(&pid.to_string()) {
                            preview.partner_name = profile.get("display_name")
                                .and_then(|v| v.as_str()).map(|s| s.to_string());
                            preview.partner_photo = profile.get("profile_photo")
                                .and_then(|v| v.as_str()).map(|s| s.to_string());
                            preview.partner_country = profile.get("country")
                                .and_then(|v| v.as_str()).map(|s| s.to_string());
                            preview.partner_online = profile.get("is_online")
                                .and_then(|v| v.as_bool()).unwrap_or(false);
                        }
                    }
                }
            }
        }
    }

    // Sort by last message time (most recent first), falling back to conversation created_at
    previews.sort_by(|a, b| {
        let a_time = a.last_message_time.unwrap_or(a.created_at);
        let b_time = b.last_message_time.unwrap_or(b.created_at);
        b_time.cmp(&a_time)
    });

    Ok(Json(ApiResponse::ok(previews)))
}

/// POST /conversations/group - create a group conversation
pub async fn create_group(
    auth_user: AuthUser,
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateGroupRequest>,
) -> AppResult<Json<ApiResponse<ConversationDetail>>> {
    let mut conn = state.db.get().map_err(|e| AppError::Internal(e.into()))?;

    if req.name.trim().is_empty() {
        return Err(AppError::new(ErrorCode::GroupNameRequired, "group name is required"));
    }

    // Create the group conversation
    let new_conv = NewConversation {
        is_group: true,
        group_name: Some(req.name),
        group_photo_url: None,
    };

    let conversation: Conversation = diesel::insert_into(conversations::table)
        .values(&new_conv)
        .get_result(&mut conn)
        .map_err(|e| AppError::Database(e))?;

    // Build member list: creator + requested members (deduplicated)
    let mut all_member_ids: Vec<Uuid> = vec![auth_user.id];
    for mid in &req.member_ids {
        if !all_member_ids.contains(mid) {
            all_member_ids.push(*mid);
        }
    }

    let new_members: Vec<NewConversationMember> = all_member_ids
        .iter()
        .map(|uid| NewConversationMember {
            conversation_id: conversation.id,
            user_id: *uid,
        })
        .collect();

    diesel::insert_into(conversation_members::table)
        .values(&new_members)
        .execute(&mut conn)
        .map_err(|e| AppError::Database(e))?;

    // Load the members back and enrich with profile data
    let raw_members: Vec<ConversationMember> = conversation_members::table
        .filter(conversation_members::conversation_id.eq(conversation.id))
        .load::<ConversationMember>(&mut conn)
        .map_err(|e| AppError::Database(e))?;

    let members = enrich_members(&state, &raw_members).await;

    Ok(Json(ApiResponse::ok(ConversationDetail {
        conversation,
        members,
    })))
}

/// GET /conversations/:id - get conversation details with members (enriched with profile data)
pub async fn get_conversation(
    auth_user: AuthUser,
    State(state): State<Arc<AppState>>,
    Path(conversation_id): Path<Uuid>,
) -> AppResult<Json<ApiResponse<ConversationDetail>>> {
    let mut conn = state.db.get().map_err(|e| AppError::Internal(e.into()))?;

    // Verify the user is a member
    let is_member: bool = conversation_members::table
        .filter(conversation_members::conversation_id.eq(conversation_id))
        .filter(conversation_members::user_id.eq(auth_user.id))
        .select(count_star())
        .first::<i64>(&mut conn)
        .map(|c| c > 0)
        .map_err(|e| AppError::Database(e))?;

    if !is_member {
        return Err(AppError::new(
            ErrorCode::NotConversationMember,
            "you are not a member of this conversation",
        ));
    }

    // Load conversation
    let conversation: Conversation = conversations::table
        .find(conversation_id)
        .first::<Conversation>(&mut conn)
        .optional()
        .map_err(|e| AppError::Database(e))?
        .ok_or_else(|| AppError::new(ErrorCode::ConversationNotFound, "conversation not found"))?;

    // Load members
    let raw_members: Vec<ConversationMember> = conversation_members::table
        .filter(conversation_members::conversation_id.eq(conversation_id))
        .load::<ConversationMember>(&mut conn)
        .map_err(|e| AppError::Database(e))?;

    // Enrich members with profile data from broz-user
    let members = enrich_members(&state, &raw_members).await;

    Ok(Json(ApiResponse::ok(ConversationDetail {
        conversation,
        members,
    })))
}

/// Fetch profile data from broz-user for a list of conversation members
async fn enrich_members(state: &AppState, raw_members: &[ConversationMember]) -> Vec<EnrichedMember> {
    let credential_ids: Vec<Uuid> = raw_members.iter().map(|m| m.user_id).collect();

    // Call broz-user internal endpoint
    let url = format!("{}/internal/profiles/batch", state.config.user_service_url);
    let client = reqwest::Client::new();
    let profiles: Vec<serde_json::Value> = match client
        .post(&url)
        .json(&serde_json::json!({ "credential_ids": credential_ids }))
        .send()
        .await
    {
        Ok(resp) => resp.json().await.unwrap_or_default(),
        Err(e) => {
            tracing::warn!(error = %e, "failed to fetch profiles from broz-user");
            vec![]
        }
    };

    // Build a lookup map: credential_id -> profile data
    let profile_map: std::collections::HashMap<String, &serde_json::Value> = profiles
        .iter()
        .filter_map(|p| {
            p.get("credential_id")
                .and_then(|v| v.as_str())
                .map(|cid| (cid.to_string(), p))
        })
        .collect();

    raw_members
        .iter()
        .map(|m| {
            let uid_str = m.user_id.to_string();
            let profile = profile_map.get(&uid_str);
            EnrichedMember {
                id: m.id,
                user_id: m.user_id,
                display_name: profile
                    .and_then(|p| p.get("display_name"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                profile_photo: profile
                    .and_then(|p| p.get("profile_photo"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                country: profile
                    .and_then(|p| p.get("country"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                is_online: profile
                    .and_then(|p| p.get("is_online"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
                joined_at: m.joined_at,
            }
        })
        .collect()
}

/// POST /conversations/:id/members - add a member to a group conversation
pub async fn add_member(
    auth_user: AuthUser,
    State(state): State<Arc<AppState>>,
    Path(conversation_id): Path<Uuid>,
    Json(req): Json<AddMemberRequest>,
) -> AppResult<Json<ApiResponse<ConversationMember>>> {
    let mut conn = state.db.get().map_err(|e| AppError::Internal(e.into()))?;

    // Load the conversation
    let conversation: Conversation = conversations::table
        .find(conversation_id)
        .first::<Conversation>(&mut conn)
        .optional()
        .map_err(|e| AppError::Database(e))?
        .ok_or_else(|| AppError::new(ErrorCode::ConversationNotFound, "conversation not found"))?;

    // Must be a group conversation
    if !conversation.is_group {
        return Err(AppError::new(
            ErrorCode::BadRequest,
            "cannot add members to a direct message conversation",
        ));
    }

    // The requester must be a member
    let is_member: bool = conversation_members::table
        .filter(conversation_members::conversation_id.eq(conversation_id))
        .filter(conversation_members::user_id.eq(auth_user.id))
        .select(count_star())
        .first::<i64>(&mut conn)
        .map(|c| c > 0)
        .map_err(|e| AppError::Database(e))?;

    if !is_member {
        return Err(AppError::new(
            ErrorCode::NotConversationMember,
            "you are not a member of this conversation",
        ));
    }

    // Check if the target user is already a member
    let already_member: bool = conversation_members::table
        .filter(conversation_members::conversation_id.eq(conversation_id))
        .filter(conversation_members::user_id.eq(req.user_id))
        .select(count_star())
        .first::<i64>(&mut conn)
        .map(|c| c > 0)
        .map_err(|e| AppError::Database(e))?;

    if already_member {
        return Err(AppError::new(
            ErrorCode::BadRequest,
            "user is already a member of this conversation",
        ));
    }

    // Insert the new member
    let new_member = NewConversationMember {
        conversation_id,
        user_id: req.user_id,
    };

    let member: ConversationMember = diesel::insert_into(conversation_members::table)
        .values(&new_member)
        .get_result(&mut conn)
        .map_err(|e| AppError::Database(e))?;

    Ok(Json(ApiResponse::ok(member)))
}

// --- Group management DTOs ---

#[derive(Debug, Deserialize)]
pub struct RenameGroupRequest {
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct GroupPhotoResponse {
    pub group_photo: String,
}

#[derive(Debug, Serialize)]
pub struct GroupNameResponse {
    pub name: String,
}

// --- Group management helpers ---

fn verify_group_membership(
    conn: &mut diesel::pg::PgConnection,
    conversation_id: Uuid,
    user_id: Uuid,
) -> AppResult<Conversation> {
    let conversation: Conversation = conversations::table
        .find(conversation_id)
        .first::<Conversation>(conn)
        .optional()
        .map_err(|e| AppError::Database(e))?
        .ok_or_else(|| AppError::new(ErrorCode::ConversationNotFound, "conversation not found"))?;

    if !conversation.is_group {
        return Err(AppError::new(ErrorCode::BadRequest, "not a group conversation"));
    }

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

    Ok(conversation)
}

/// POST /conversations/group/:id/photo - upload group photo
pub async fn update_group_photo(
    auth_user: AuthUser,
    State(state): State<Arc<AppState>>,
    Path(conversation_id): Path<Uuid>,
    mut multipart: Multipart,
) -> AppResult<Json<ApiResponse<GroupPhotoResponse>>> {
    let mut conn = state.db.get().map_err(|e| AppError::Internal(e.into()))?;
    verify_group_membership(&mut conn, conversation_id, auth_user.id)?;

    // Extract file from multipart
    let mut file_data: Option<(Vec<u8>, String)> = None;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::new(ErrorCode::ValidationError, format!("multipart error: {e}")))?
    {
        let name = field.name().unwrap_or("").to_string();
        if name == "file" {
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
    }

    let (data, content_type) = file_data
        .ok_or_else(|| AppError::new(ErrorCode::ValidationError, "no file provided"))?;

    // Validate image type
    let ext = match content_type.as_str() {
        "image/jpeg" | "image/jpg" => "jpg",
        "image/png" => "png",
        "image/webp" => "webp",
        _ => {
            return Err(AppError::new(
                ErrorCode::ValidationError,
                "unsupported format, accepted: jpeg, png, webp",
            ));
        }
    };

    // Upload to MinIO
    let file_id = Uuid::now_v7();
    let key = format!("groups/{}/{}.{}", conversation_id, file_id, ext);
    let photo_url = state
        .minio
        .upload(&key, data, &content_type)
        .await
        .map_err(|e| AppError::new(ErrorCode::ValidationError, e))?;

    // Update conversation
    diesel::update(conversations::table.find(conversation_id))
        .set(conversations::group_photo_url.eq(&photo_url))
        .execute(&mut conn)
        .map_err(|e| AppError::Database(e))?;

    // Broadcast socket event to group members
    let member_ids: Vec<Uuid> = conversation_members::table
        .filter(conversation_members::conversation_id.eq(conversation_id))
        .select(conversation_members::user_id)
        .load::<Uuid>(&mut conn)
        .map_err(|e| AppError::Database(e))?;

    for mid in &member_ids {
        let room = format!("user:{}", mid);
        let _ = state.io.to(room).emit(
            "group-photo-updated",
            &serde_json::json!({
                "conversation_id": conversation_id,
                "group_photo": photo_url,
            }),
        );
    }

    Ok(Json(ApiResponse::ok(GroupPhotoResponse {
        group_photo: photo_url,
    })))
}

/// PUT /conversations/group/:id/name - rename a group
pub async fn rename_group(
    auth_user: AuthUser,
    State(state): State<Arc<AppState>>,
    Path(conversation_id): Path<Uuid>,
    Json(req): Json<RenameGroupRequest>,
) -> AppResult<Json<ApiResponse<GroupNameResponse>>> {
    let name = req.name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::new(ErrorCode::GroupNameRequired, "group name is required"));
    }
    if name.len() > 50 {
        return Err(AppError::new(ErrorCode::ValidationError, "group name max 50 characters"));
    }

    let mut conn = state.db.get().map_err(|e| AppError::Internal(e.into()))?;
    verify_group_membership(&mut conn, conversation_id, auth_user.id)?;

    // Update conversation name
    diesel::update(conversations::table.find(conversation_id))
        .set(conversations::group_name.eq(&name))
        .execute(&mut conn)
        .map_err(|e| AppError::Database(e))?;

    // Broadcast socket event to group members
    let member_ids: Vec<Uuid> = conversation_members::table
        .filter(conversation_members::conversation_id.eq(conversation_id))
        .select(conversation_members::user_id)
        .load::<Uuid>(&mut conn)
        .map_err(|e| AppError::Database(e))?;

    for mid in &member_ids {
        let room = format!("user:{}", mid);
        let _ = state.io.to(room).emit(
            "group-renamed",
            &serde_json::json!({
                "conversation_id": conversation_id,
                "name": name,
            }),
        );
    }

    Ok(Json(ApiResponse::ok(GroupNameResponse { name })))
}
