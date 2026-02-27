use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::Json;
use uuid::Uuid;

use broz_shared::errors::AppResult;
use broz_shared::types::api::ApiResponse;
use broz_shared::types::auth::AuthUser;
use broz_shared::types::pagination::{Paginated, PaginationParams};

use crate::models::Notification;
use crate::services::notification_service;
use crate::AppState;

/// GET /notifications
/// List notifications for the authenticated user with pagination.
pub async fn list_notifications(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Query(params): Query<PaginationParams>,
) -> AppResult<Json<ApiResponse<Paginated<Notification>>>> {
    let limit = params.limit() as i64;
    let offset = params.offset() as i64;

    let (items, total) = notification_service::list_notifications(
        &state.db,
        auth_user.id,
        limit,
        offset,
    )?;

    let paginated = Paginated::new(items, total as u64, &params);
    Ok(Json(ApiResponse::ok(paginated)))
}

/// GET /notifications/unread-count
/// Get the count of unread notifications for the authenticated user.
pub async fn unread_count(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
) -> AppResult<Json<ApiResponse<UnreadCountResponse>>> {
    let count = notification_service::count_unread(&state.db, auth_user.id)?;

    Ok(Json(ApiResponse::ok(UnreadCountResponse { count })))
}

#[derive(Debug, serde::Serialize)]
pub struct UnreadCountResponse {
    pub count: i64,
}

/// POST /notifications/mark-all-read
/// Mark all unread notifications as read for the authenticated user.
pub async fn mark_all_read(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
) -> AppResult<Json<ApiResponse<MarkAllReadResponse>>> {
    let updated = notification_service::mark_all_read(&state.db, auth_user.id)?;

    Ok(Json(ApiResponse::ok(MarkAllReadResponse { updated })))
}

#[derive(Debug, serde::Serialize)]
pub struct MarkAllReadResponse {
    pub updated: usize,
}

/// POST /notifications/:id/read
/// Mark a single notification as read.
pub async fn mark_read(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<ApiResponse<Notification>>> {
    let notification = notification_service::mark_read(&state.db, id, auth_user.id)?;

    Ok(Json(ApiResponse::ok(notification)))
}
