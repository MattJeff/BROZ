use std::sync::Arc;

use axum::extract::{Path, State};
use axum::Json;
use chrono::{Duration, Utc};
use diesel::prelude::*;
use serde::Deserialize;
use uuid::Uuid;

use broz_shared::errors::{AppError, AppResult, ErrorCode};
use broz_shared::types::api::ApiResponse;
use broz_shared::types::auth::AuthUser;

use crate::events::publisher;
use crate::models::{LiveCamRequest, NewLiveCamRequest};
use crate::schema::livecam_requests;
use crate::AppState;

// ---------------------------------------------------------------------------
// Request / response types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct CreateLiveCamRequestPayload {
    pub target_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct RespondLiveCamPayload {
    pub accepted: bool,
}

// ---------------------------------------------------------------------------
// POST /livecam/request
// ---------------------------------------------------------------------------

pub async fn create_livecam_request(
    auth_user: AuthUser,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateLiveCamRequestPayload>,
) -> AppResult<Json<ApiResponse<LiveCamRequest>>> {
    if auth_user.id == payload.target_id {
        return Err(AppError::bad_request("Cannot send livecam request to yourself"));
    }

    let expires_at = Utc::now() + Duration::seconds(60);

    let new_request = NewLiveCamRequest {
        requester_id: auth_user.id,
        target_id: payload.target_id,
        expires_at,
    };

    let mut conn = state
        .db
        .get()
        .map_err(|e| AppError::internal(format!("database connection error: {e}")))?;

    let request = diesel::insert_into(livecam_requests::table)
        .values(&new_request)
        .get_result::<LiveCamRequest>(&mut conn)
        .map_err(AppError::Database)?;

    // Store in Redis for quick lookup with TTL
    let redis_key = format!("livecam:request:{}", request.id);
    let redis_val = serde_json::json!({
        "id": request.id,
        "requester_id": auth_user.id,
        "target_id": payload.target_id,
        "status": "pending",
    })
    .to_string();
    let _ = state.redis.set(&redis_key, &redis_val, 60).await;

    // Publish event
    publisher::publish_livecam_requested(
        &state.rabbitmq,
        request.id,
        auth_user.id,
        payload.target_id,
    )
    .await;

    Ok(Json(ApiResponse::ok(request)))
}

// ---------------------------------------------------------------------------
// PUT /livecam/:id/respond
// ---------------------------------------------------------------------------

pub async fn respond_livecam_request(
    auth_user: AuthUser,
    State(state): State<Arc<AppState>>,
    Path(request_id): Path<Uuid>,
    Json(payload): Json<RespondLiveCamPayload>,
) -> AppResult<Json<ApiResponse<LiveCamRequest>>> {
    let mut conn = state
        .db
        .get()
        .map_err(|e| AppError::internal(format!("database connection error: {e}")))?;

    // Find the request
    let request = livecam_requests::table
        .find(request_id)
        .first::<LiveCamRequest>(&mut conn)
        .map_err(|_| AppError::new(ErrorCode::LiveCamRequestNotFound, "LiveCam request not found"))?;

    // Verify the target is the authenticated user
    if request.target_id != auth_user.id {
        return Err(AppError::forbidden("You can only respond to requests targeting you"));
    }

    // Check if already responded
    if request.status != "pending" {
        return Err(AppError::bad_request("This request has already been responded to"));
    }

    // Check if expired
    if Utc::now() > request.expires_at {
        // Update status to expired
        let _ = diesel::update(livecam_requests::table.find(request_id))
            .set(livecam_requests::status.eq("expired"))
            .execute(&mut conn);

        return Err(AppError::new(
            ErrorCode::LiveCamRequestExpired,
            "This LiveCam request has expired",
        ));
    }

    let new_status = if payload.accepted { "accepted" } else { "declined" };
    let room_id = if payload.accepted {
        Some(format!("livecam:{}:{}", request.requester_id, request.target_id))
    } else {
        None
    };

    let updated = diesel::update(livecam_requests::table.find(request_id))
        .set((
            livecam_requests::status.eq(new_status),
            livecam_requests::room_id.eq(&room_id),
            livecam_requests::responded_at.eq(Some(Utc::now())),
        ))
        .get_result::<LiveCamRequest>(&mut conn)
        .map_err(AppError::Database)?;

    // Clean up Redis
    let redis_key = format!("livecam:request:{}", request_id);
    let _ = state.redis.del(&redis_key).await;

    // Publish event
    publisher::publish_livecam_responded(
        &state.rabbitmq,
        request_id,
        request.requester_id,
        request.target_id,
        payload.accepted,
    )
    .await;

    Ok(Json(ApiResponse::ok(updated)))
}

// ---------------------------------------------------------------------------
// GET /livecam/pending
// ---------------------------------------------------------------------------

pub async fn get_pending_requests(
    auth_user: AuthUser,
    State(state): State<Arc<AppState>>,
) -> AppResult<Json<ApiResponse<Vec<LiveCamRequest>>>> {
    let mut conn = state
        .db
        .get()
        .map_err(|e| AppError::internal(format!("database connection error: {e}")))?;

    let now = Utc::now();

    let requests = livecam_requests::table
        .filter(livecam_requests::target_id.eq(auth_user.id))
        .filter(livecam_requests::status.eq("pending"))
        .filter(livecam_requests::expires_at.gt(now))
        .order(livecam_requests::created_at.desc())
        .load::<LiveCamRequest>(&mut conn)
        .map_err(AppError::Database)?;

    Ok(Json(ApiResponse::ok(requests)))
}
