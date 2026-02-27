use axum::extract::{Path, State};
use axum::Json;
use chrono::Utc;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::schema::{follows, profiles};
use crate::models::Profile;
use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct UpdatePresenceRequest {
    pub user_id: Uuid,
    pub is_online: bool,
}

#[derive(Debug, Serialize)]
pub struct PresenceResponse {
    pub ok: bool,
}

/// POST /internal/presence — Update is_online + last_seen_at in DB (service-to-service, no auth)
pub async fn update_presence(
    State(state): State<Arc<AppState>>,
    Json(req): Json<UpdatePresenceRequest>,
) -> Json<PresenceResponse> {
    let mut conn = match state.db.get() {
        Ok(c) => c,
        Err(e) => {
            tracing::error!(error = %e, "failed to get db connection for presence update");
            return Json(PresenceResponse { ok: false });
        }
    };

    let result = if req.is_online {
        diesel::update(profiles::table.filter(profiles::credential_id.eq(req.user_id)))
            .set(profiles::is_online.eq(true))
            .execute(&mut conn)
    } else {
        diesel::update(profiles::table.filter(profiles::credential_id.eq(req.user_id)))
            .set((
                profiles::is_online.eq(false),
                profiles::last_seen_at.eq(Some(Utc::now())),
            ))
            .execute(&mut conn)
    };

    match result {
        Ok(_) => {
            tracing::debug!(user_id = %req.user_id, is_online = req.is_online, "presence updated");
            Json(PresenceResponse { ok: true })
        }
        Err(e) => {
            tracing::error!(error = %e, user_id = %req.user_id, "failed to update presence");
            Json(PresenceResponse { ok: false })
        }
    }
}

#[derive(Debug, Serialize)]
pub struct FollowerIdsResponse {
    pub follower_ids: Vec<Uuid>,
}

/// GET /internal/follower-ids/:id — Return credential_ids of accepted followers (service-to-service, no auth)
pub async fn get_follower_ids(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<Uuid>,
) -> Json<FollowerIdsResponse> {
    let mut conn = match state.db.get() {
        Ok(c) => c,
        Err(e) => {
            tracing::error!(error = %e, "failed to get db connection for follower ids");
            return Json(FollowerIdsResponse { follower_ids: vec![] });
        }
    };

    // Find profile by credential_id
    let profile_id: Option<Uuid> = profiles::table
        .filter(profiles::credential_id.eq(user_id))
        .select(profiles::id)
        .first(&mut conn)
        .optional()
        .unwrap_or(None);

    let profile_id = match profile_id {
        Some(id) => id,
        None => return Json(FollowerIdsResponse { follower_ids: vec![] }),
    };

    // Get follower profile IDs (accepted follows where this user is the following_id)
    let follower_profile_ids: Vec<Uuid> = follows::table
        .filter(follows::following_id.eq(profile_id))
        .filter(follows::status.eq("accepted"))
        .select(follows::follower_id)
        .load::<Uuid>(&mut conn)
        .unwrap_or_default();

    if follower_profile_ids.is_empty() {
        return Json(FollowerIdsResponse { follower_ids: vec![] });
    }

    // Convert profile IDs to credential_ids
    let credential_ids: Vec<Uuid> = profiles::table
        .filter(profiles::id.eq_any(&follower_profile_ids))
        .select(profiles::credential_id)
        .load::<Uuid>(&mut conn)
        .unwrap_or_default();

    Json(FollowerIdsResponse { follower_ids: credential_ids })
}

// --- Batch profiles lookup ---

#[derive(Debug, Deserialize)]
pub struct BatchProfilesRequest {
    pub credential_ids: Vec<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct BatchProfileEntry {
    pub credential_id: Uuid,
    pub display_name: Option<String>,
    pub profile_photo: Option<String>,
    pub country: Option<String>,
    pub is_online: bool,
}

/// POST /internal/profiles/batch — Return profile info for a list of credential_ids (service-to-service, no auth)
pub async fn batch_profiles(
    State(state): State<Arc<AppState>>,
    Json(req): Json<BatchProfilesRequest>,
) -> Json<Vec<BatchProfileEntry>> {
    let mut conn = match state.db.get() {
        Ok(c) => c,
        Err(e) => {
            tracing::error!(error = %e, "failed to get db connection for batch profiles");
            return Json(vec![]);
        }
    };

    let found: Vec<Profile> = profiles::table
        .filter(profiles::credential_id.eq_any(&req.credential_ids))
        .load::<Profile>(&mut conn)
        .unwrap_or_default();

    // Enrich is_online from Redis
    let mut entries: Vec<BatchProfileEntry> = Vec::with_capacity(found.len());
    for p in found {
        let redis_online = {
            let key = format!("online:{}", p.credential_id);
            state.redis.exists(&key).await.unwrap_or(false)
        };
        entries.push(BatchProfileEntry {
            credential_id: p.credential_id,
            display_name: p.display_name,
            profile_photo: p.profile_photo_url,
            country: p.country,
            is_online: redis_online || p.is_online,
        });
    }

    Json(entries)
}
