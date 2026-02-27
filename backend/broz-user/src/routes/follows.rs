use axum::extract::{Path, State};
use axum::Json;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use broz_shared::errors::{AppError, AppResult, ErrorCode};
use broz_shared::types::auth::AuthUser;
use broz_shared::types::ApiResponse;

use crate::events::publisher;
use crate::models::{Follow, NewFollow, Profile};
use crate::schema::{follows, profiles};
use crate::AppState;

// --- POST /follows/:id ---

pub async fn send_follow_request(
    user: AuthUser,
    State(state): State<Arc<AppState>>,
    Path(target_id): Path<Uuid>,
) -> AppResult<Json<ApiResponse<Follow>>> {
    let mut conn = state.db.get().map_err(|e| AppError::internal(e.to_string()))?;

    // Get follower profile
    let follower = profiles::table
        .filter(profiles::credential_id.eq(user.id))
        .first::<Profile>(&mut conn)
        .map_err(|_| AppError::new(ErrorCode::ProfileNotFound, "profile not found"))?;

    // Check target exists (target_id can be profile id or credential_id)
    let target = profiles::table
        .filter(
            profiles::id.eq(target_id)
                .or(profiles::credential_id.eq(target_id)),
        )
        .first::<Profile>(&mut conn)
        .map_err(|_| AppError::new(ErrorCode::ProfileNotFound, "target profile not found"))?;

    // Cannot follow self
    if follower.id == target.id {
        return Err(AppError::new(ErrorCode::CannotFollowSelf, "cannot follow yourself"));
    }

    // Check for existing follow
    let existing: bool = follows::table
        .filter(follows::follower_id.eq(follower.id))
        .filter(follows::following_id.eq(target.id))
        .count()
        .get_result::<i64>(&mut conn)
        .map(|c| c > 0)
        .unwrap_or(false);

    if existing {
        return Err(AppError::new(ErrorCode::FollowAlreadyExists, "follow request already exists"));
    }

    let new_follow = NewFollow {
        follower_id: follower.id,
        following_id: target.id,
    };

    let follow = diesel::insert_into(follows::table)
        .values(&new_follow)
        .get_result::<Follow>(&mut conn)?;

    // Auto-accept if mutual: if the target already sent a follow to us, accept both
    let reverse_follow: Option<Follow> = follows::table
        .filter(follows::follower_id.eq(target.id))
        .filter(follows::following_id.eq(follower.id))
        .filter(follows::status.eq("pending"))
        .first::<Follow>(&mut conn)
        .optional()?;

    let follow = if let Some(reverse) = reverse_follow {
        // Accept both follows
        diesel::update(follows::table.filter(follows::id.eq(reverse.id)))
            .set(follows::status.eq("accepted"))
            .execute(&mut conn)?;
        let updated = diesel::update(follows::table.filter(follows::id.eq(follow.id)))
            .set(follows::status.eq("accepted"))
            .get_result::<Follow>(&mut conn)?;
        publisher::publish_follow_accepted(&state.rabbitmq, follower.credential_id, target.credential_id).await;
        publisher::publish_follow_accepted(&state.rabbitmq, target.credential_id, follower.credential_id).await;
        updated
    } else {
        let display_name = follower.display_name.as_deref().unwrap_or("unknown");
        publisher::publish_follow_requested(&state.rabbitmq, follower.id, target.id, display_name).await;
        follow
    };

    Ok(Json(ApiResponse::ok(follow)))
}

// --- PUT /follows/:id/respond ---

#[derive(Debug, Deserialize)]
pub struct RespondFollowRequest {
    pub accepted: bool,
}

pub async fn respond_follow(
    user: AuthUser,
    State(state): State<Arc<AppState>>,
    Path(follow_id): Path<Uuid>,
    Json(req): Json<RespondFollowRequest>,
) -> AppResult<Json<ApiResponse<Follow>>> {
    let mut conn = state.db.get().map_err(|e| AppError::internal(e.to_string()))?;

    // Get current user profile
    let profile = profiles::table
        .filter(profiles::credential_id.eq(user.id))
        .first::<Profile>(&mut conn)
        .map_err(|_| AppError::new(ErrorCode::ProfileNotFound, "profile not found"))?;

    // Get the follow request
    let follow = follows::table
        .find(follow_id)
        .first::<Follow>(&mut conn)
        .map_err(|_| AppError::new(ErrorCode::FollowNotFound, "follow request not found"))?;

    // Only the following_id user (the one being followed) can respond
    if follow.following_id != profile.id {
        return Err(AppError::new(ErrorCode::Forbidden, "only the target user can respond to follow requests"));
    }

    let new_status = if req.accepted { "accepted" } else { "rejected" };

    let updated = diesel::update(follows::table.filter(follows::id.eq(follow_id)))
        .set(follows::status.eq(new_status))
        .get_result::<Follow>(&mut conn)?;

    if req.accepted {
        // Resolve credential_ids from profile_ids for the event
        let follower_cred: Uuid = profiles::table
            .find(follow.follower_id)
            .select(profiles::credential_id)
            .first(&mut conn)
            .unwrap_or(follow.follower_id);
        let following_cred: Uuid = profiles::table
            .find(follow.following_id)
            .select(profiles::credential_id)
            .first(&mut conn)
            .unwrap_or(follow.following_id);
        publisher::publish_follow_accepted(&state.rabbitmq, follower_cred, following_cred).await;
    }

    Ok(Json(ApiResponse::ok(updated)))
}

// --- DELETE /follows/:id ---

pub async fn remove_follow(
    user: AuthUser,
    State(state): State<Arc<AppState>>,
    Path(follow_id): Path<Uuid>,
) -> AppResult<Json<ApiResponse<FollowRemovedResponse>>> {
    let mut conn = state.db.get().map_err(|e| AppError::internal(e.to_string()))?;

    // Get current user profile
    let profile = profiles::table
        .filter(profiles::credential_id.eq(user.id))
        .first::<Profile>(&mut conn)
        .map_err(|_| AppError::new(ErrorCode::ProfileNotFound, "profile not found"))?;

    // Get the follow
    let follow = follows::table
        .find(follow_id)
        .first::<Follow>(&mut conn)
        .map_err(|_| AppError::new(ErrorCode::FollowNotFound, "follow not found"))?;

    // Either party can remove
    if follow.follower_id != profile.id && follow.following_id != profile.id {
        return Err(AppError::new(ErrorCode::Forbidden, "you are not part of this follow relationship"));
    }

    diesel::delete(follows::table.filter(follows::id.eq(follow_id)))
        .execute(&mut conn)?;

    publisher::publish_follow_removed(&state.rabbitmq, follow.follower_id, follow.following_id).await;

    Ok(Json(ApiResponse::ok(FollowRemovedResponse { removed: true })))
}

#[derive(Debug, Serialize)]
pub struct FollowRemovedResponse {
    pub removed: bool,
}

// --- GET /followers ---

pub async fn list_followers(
    user: AuthUser,
    State(state): State<Arc<AppState>>,
) -> AppResult<Json<ApiResponse<Vec<Profile>>>> {
    let mut conn = state.db.get().map_err(|e| AppError::internal(e.to_string()))?;

    let profile = profiles::table
        .filter(profiles::credential_id.eq(user.id))
        .first::<Profile>(&mut conn)
        .map_err(|_| AppError::new(ErrorCode::ProfileNotFound, "profile not found"))?;

    // Get follower profile IDs where status = accepted, most recent first
    let follower_ids: Vec<Uuid> = follows::table
        .filter(follows::following_id.eq(profile.id))
        .filter(follows::status.eq("accepted"))
        .order(follows::created_at.desc())
        .select(follows::follower_id)
        .load::<Uuid>(&mut conn)?;

    let mut followers = profiles::table
        .filter(profiles::id.eq_any(&follower_ids))
        .load::<Profile>(&mut conn)?;

    // Preserve order from follows query (most recent first)
    let id_order: std::collections::HashMap<uuid::Uuid, usize> = follower_ids.iter().enumerate().map(|(i, id)| (*id, i)).collect();
    followers.sort_by_key(|p| id_order.get(&p.id).copied().unwrap_or(usize::MAX));

    // Enrich is_online from Redis (source of truth for presence)
    for p in &mut followers {
        let key = format!("online:{}", p.credential_id);
        if let Ok(true) = state.redis.exists(&key).await {
            p.is_online = true;
        }
    }

    Ok(Json(ApiResponse::ok(followers)))
}

// --- GET /following ---

pub async fn list_following(
    user: AuthUser,
    State(state): State<Arc<AppState>>,
) -> AppResult<Json<ApiResponse<Vec<Profile>>>> {
    let mut conn = state.db.get().map_err(|e| AppError::internal(e.to_string()))?;

    let profile = profiles::table
        .filter(profiles::credential_id.eq(user.id))
        .first::<Profile>(&mut conn)
        .map_err(|_| AppError::new(ErrorCode::ProfileNotFound, "profile not found"))?;

    // Get following profile IDs where status = accepted, most recent first
    let following_ids: Vec<Uuid> = follows::table
        .filter(follows::follower_id.eq(profile.id))
        .filter(follows::status.eq("accepted"))
        .order(follows::created_at.desc())
        .select(follows::following_id)
        .load::<Uuid>(&mut conn)?;

    let mut following = profiles::table
        .filter(profiles::id.eq_any(&following_ids))
        .load::<Profile>(&mut conn)?;

    // Preserve order from follows query (most recent first)
    let id_order: std::collections::HashMap<uuid::Uuid, usize> = following_ids.iter().enumerate().map(|(i, id)| (*id, i)).collect();
    following.sort_by_key(|p| id_order.get(&p.id).copied().unwrap_or(usize::MAX));

    // Enrich is_online from Redis (source of truth for presence)
    for p in &mut following {
        let key = format!("online:{}", p.credential_id);
        if let Ok(true) = state.redis.exists(&key).await {
            p.is_online = true;
        }
    }

    Ok(Json(ApiResponse::ok(following)))
}
