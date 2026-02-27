use axum::extract::{Path, Query, State};
use axum::Json;
use chrono::{Duration, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use broz_shared::errors::{AppError, AppResult, ErrorCode};
use broz_shared::middleware::AdminUser;
use broz_shared::types::api::ApiResponse;
use broz_shared::types::pagination::{Paginated, PaginationParams};

use crate::events::publisher;
use crate::models::{AdminAction, NewAdminAction, NewSanction, Report, Sanction};
use crate::schema::{admin_actions, reports, sanctions};
use crate::AppState;

// --- Request / Response types ---

#[derive(Debug, Deserialize)]
pub struct ReportFilterParams {
    #[serde(default = "default_page")]
    pub page: u64,
    #[serde(default = "default_per_page")]
    pub per_page: u64,
    pub status: Option<String>,
}

fn default_page() -> u64 { 1 }
fn default_per_page() -> u64 { 20 }

impl ReportFilterParams {
    fn pagination(&self) -> PaginationParams {
        PaginationParams {
            page: self.page,
            per_page: self.per_page,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ReviewReportRequest {
    pub status: String, // "actioned" or "dismissed"
    pub sanction_type: Option<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct IssueSanctionRequest {
    pub sanction_type: String,
    pub reason: String,
    pub expires_at: Option<chrono::DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct DashboardStats {
    pub pending_reports: i64,
    pub active_sanctions: i64,
    pub reports_today: i64,
}

// --- List reports (paginated, optional status filter) ---

pub async fn list_reports(
    State(state): State<Arc<AppState>>,
    _admin: AdminUser,
    Query(params): Query<ReportFilterParams>,
) -> AppResult<Json<ApiResponse<Paginated<Report>>>> {
    let mut conn = state.db.get()
        .map_err(|e| AppError::internal(format!("db pool error: {e}")))?;

    let pagination = params.pagination();
    let offset = pagination.offset() as i64;
    let limit = pagination.limit() as i64;

    let (items, total): (Vec<Report>, i64) = if let Some(ref status) = params.status {
        let items = reports::table
            .filter(reports::status.eq(status))
            .order(reports::created_at.desc())
            .offset(offset)
            .limit(limit)
            .load::<Report>(&mut conn)
            .map_err(|e| AppError::internal(format!("db error: {e}")))?;

        let total: i64 = reports::table
            .filter(reports::status.eq(status))
            .count()
            .get_result(&mut conn)
            .map_err(|e| AppError::internal(format!("db error: {e}")))?;

        (items, total)
    } else {
        let items = reports::table
            .order(reports::created_at.desc())
            .offset(offset)
            .limit(limit)
            .load::<Report>(&mut conn)
            .map_err(|e| AppError::internal(format!("db error: {e}")))?;

        let total: i64 = reports::table
            .count()
            .get_result(&mut conn)
            .map_err(|e| AppError::internal(format!("db error: {e}")))?;

        (items, total)
    };

    let paginated = Paginated::new(items, total as u64, &pagination);
    Ok(Json(ApiResponse::ok(paginated)))
}

// --- Get report details ---

pub async fn get_report(
    State(state): State<Arc<AppState>>,
    _admin: AdminUser,
    Path(report_id): Path<Uuid>,
) -> AppResult<Json<ApiResponse<Report>>> {
    let mut conn = state.db.get()
        .map_err(|e| AppError::internal(format!("db pool error: {e}")))?;

    let report = reports::table
        .find(report_id)
        .first::<Report>(&mut conn)
        .optional()
        .map_err(|e| AppError::internal(format!("db error: {e}")))?
        .ok_or_else(|| AppError::new(ErrorCode::ReportNotFound, "report not found"))?;

    Ok(Json(ApiResponse::ok(report)))
}

// --- Review report ---

pub async fn review_report(
    State(state): State<Arc<AppState>>,
    admin: AdminUser,
    Path(report_id): Path<Uuid>,
    Json(body): Json<ReviewReportRequest>,
) -> AppResult<Json<ApiResponse<Report>>> {
    if body.status != "actioned" && body.status != "dismissed" {
        return Err(AppError::new(
            ErrorCode::ValidationError,
            "status must be 'actioned' or 'dismissed'",
        ));
    }

    let mut conn = state.db.get()
        .map_err(|e| AppError::internal(format!("db pool error: {e}")))?;

    // Load the report
    let report = reports::table
        .find(report_id)
        .first::<Report>(&mut conn)
        .optional()
        .map_err(|e| AppError::internal(format!("db error: {e}")))?
        .ok_or_else(|| AppError::new(ErrorCode::ReportNotFound, "report not found"))?;

    if report.status != "pending" {
        return Err(AppError::new(
            ErrorCode::ReportAlreadyReviewed,
            "this report has already been reviewed",
        ));
    }

    // Update report status
    let updated_report: Report = diesel::update(reports::table.find(report_id))
        .set((
            reports::status.eq(&body.status),
            reports::reviewed_by.eq(admin.0.id),
            reports::reviewed_at.eq(Utc::now()),
        ))
        .get_result(&mut conn)
        .map_err(|e| AppError::internal(format!("failed to update report: {e}")))?;

    // If actioned with sanction, create the sanction
    if body.status == "actioned" {
        if let Some(ref sanction_type) = body.sanction_type {
            let reason = body.reason.clone().unwrap_or_else(|| report.reason.clone());
            let expires_at = compute_expires_at(sanction_type);

            let new_sanction = NewSanction {
                user_id: report.reported_id,
                report_id: Some(report_id),
                sanction_type: sanction_type.clone(),
                reason: reason.clone(),
                issued_by: admin.0.id,
                expires_at,
            };

            let sanction: Sanction = diesel::insert_into(sanctions::table)
                .values(&new_sanction)
                .get_result(&mut conn)
                .map_err(|e| AppError::internal(format!("failed to create sanction: {e}")))?;

            // Publish sanction_issued event
            publisher::publish_sanction_issued(
                &state.rabbitmq,
                sanction.id,
                sanction.user_id,
                &sanction.sanction_type,
                &sanction.reason,
                sanction.expires_at,
            )
            .await;
        }
    }

    // Log admin action
    let action_detail = serde_json::json!({
        "report_id": report_id,
        "status": body.status,
        "sanction_type": body.sanction_type,
    });

    let admin_action = NewAdminAction {
        admin_id: admin.0.id,
        action: format!("review_report_{}", body.status),
        target_user_id: Some(report.reported_id),
        details: Some(action_detail),
    };

    diesel::insert_into(admin_actions::table)
        .values(&admin_action)
        .execute(&mut conn)
        .map_err(|e| AppError::internal(format!("failed to log admin action: {e}")))?;

    Ok(Json(ApiResponse::ok(updated_report)))
}

// --- Get user sanction history ---

pub async fn get_user_sanctions(
    State(state): State<Arc<AppState>>,
    _admin: AdminUser,
    Path(user_id): Path<Uuid>,
) -> AppResult<Json<ApiResponse<Vec<Sanction>>>> {
    let mut conn = state.db.get()
        .map_err(|e| AppError::internal(format!("db pool error: {e}")))?;

    let user_sanctions = sanctions::table
        .filter(sanctions::user_id.eq(user_id))
        .order(sanctions::created_at.desc())
        .load::<Sanction>(&mut conn)
        .map_err(|e| AppError::internal(format!("db error: {e}")))?;

    Ok(Json(ApiResponse::ok(user_sanctions)))
}

// --- Issue sanction directly ---

pub async fn issue_sanction(
    State(state): State<Arc<AppState>>,
    admin: AdminUser,
    Path(user_id): Path<Uuid>,
    Json(body): Json<IssueSanctionRequest>,
) -> AppResult<Json<ApiResponse<Sanction>>> {
    // Validate sanction type
    let valid_types = ["warning", "ban_1h", "ban_24h", "ban_30d", "ban_permanent"];
    if !valid_types.contains(&body.sanction_type.as_str()) {
        return Err(AppError::new(
            ErrorCode::ValidationError,
            format!(
                "invalid sanction_type '{}'. Must be one of: {}",
                body.sanction_type,
                valid_types.join(", ")
            ),
        ));
    }

    let mut conn = state.db.get()
        .map_err(|e| AppError::internal(format!("db pool error: {e}")))?;

    let expires_at = body.expires_at.or_else(|| compute_expires_at(&body.sanction_type));

    let new_sanction = NewSanction {
        user_id,
        report_id: None,
        sanction_type: body.sanction_type.clone(),
        reason: body.reason.clone(),
        issued_by: admin.0.id,
        expires_at,
    };

    let sanction: Sanction = diesel::insert_into(sanctions::table)
        .values(&new_sanction)
        .get_result(&mut conn)
        .map_err(|e| AppError::internal(format!("failed to create sanction: {e}")))?;

    // Publish sanction_issued event
    publisher::publish_sanction_issued(
        &state.rabbitmq,
        sanction.id,
        sanction.user_id,
        &sanction.sanction_type,
        &sanction.reason,
        sanction.expires_at,
    )
    .await;

    // Log admin action
    let action_detail = serde_json::json!({
        "sanction_id": sanction.id,
        "sanction_type": body.sanction_type,
        "reason": body.reason,
        "expires_at": sanction.expires_at,
    });

    let admin_action = NewAdminAction {
        admin_id: admin.0.id,
        action: "issue_sanction".to_string(),
        target_user_id: Some(user_id),
        details: Some(action_detail),
    };

    diesel::insert_into(admin_actions::table)
        .values(&admin_action)
        .execute(&mut conn)
        .map_err(|e| AppError::internal(format!("failed to log admin action: {e}")))?;

    Ok(Json(ApiResponse::ok(sanction)))
}

// --- Lift sanction ---

pub async fn lift_sanction(
    State(state): State<Arc<AppState>>,
    admin: AdminUser,
    Path((user_id, sanction_id)): Path<(Uuid, Uuid)>,
) -> AppResult<Json<ApiResponse<Sanction>>> {
    let mut conn = state.db.get()
        .map_err(|e| AppError::internal(format!("db pool error: {e}")))?;

    // Load the sanction and verify it belongs to this user
    let sanction = sanctions::table
        .find(sanction_id)
        .first::<Sanction>(&mut conn)
        .optional()
        .map_err(|e| AppError::internal(format!("db error: {e}")))?
        .ok_or_else(|| AppError::new(ErrorCode::SanctionNotFound, "sanction not found"))?;

    if sanction.user_id != user_id {
        return Err(AppError::new(
            ErrorCode::SanctionNotFound,
            "sanction not found for this user",
        ));
    }

    // Set is_active = false
    let updated: Sanction = diesel::update(sanctions::table.find(sanction_id))
        .set(sanctions::is_active.eq(false))
        .get_result(&mut conn)
        .map_err(|e| AppError::internal(format!("failed to update sanction: {e}")))?;

    // Publish sanction_lifted event
    publisher::publish_sanction_lifted(&state.rabbitmq, sanction_id, user_id).await;

    // Log admin action
    let action_detail = serde_json::json!({
        "sanction_id": sanction_id,
        "sanction_type": sanction.sanction_type,
    });

    let admin_action = NewAdminAction {
        admin_id: admin.0.id,
        action: "lift_sanction".to_string(),
        target_user_id: Some(user_id),
        details: Some(action_detail),
    };

    diesel::insert_into(admin_actions::table)
        .values(&admin_action)
        .execute(&mut conn)
        .map_err(|e| AppError::internal(format!("failed to log admin action: {e}")))?;

    Ok(Json(ApiResponse::ok(updated)))
}

// --- List active sanctions (paginated) ---

pub async fn list_active_sanctions(
    State(state): State<Arc<AppState>>,
    _admin: AdminUser,
    Query(params): Query<PaginationParams>,
) -> AppResult<Json<ApiResponse<Paginated<Sanction>>>> {
    let mut conn = state.db.get()
        .map_err(|e| AppError::internal(format!("db pool error: {e}")))?;

    let offset = params.offset() as i64;
    let limit = params.limit() as i64;

    let items = sanctions::table
        .filter(sanctions::is_active.eq(true))
        .order(sanctions::created_at.desc())
        .offset(offset)
        .limit(limit)
        .load::<Sanction>(&mut conn)
        .map_err(|e| AppError::internal(format!("db error: {e}")))?;

    let total: i64 = sanctions::table
        .filter(sanctions::is_active.eq(true))
        .count()
        .get_result(&mut conn)
        .map_err(|e| AppError::internal(format!("db error: {e}")))?;

    let paginated = Paginated::new(items, total as u64, &params);
    Ok(Json(ApiResponse::ok(paginated)))
}

// --- Dashboard stats ---

pub async fn get_stats(
    State(state): State<Arc<AppState>>,
    _admin: AdminUser,
) -> AppResult<Json<ApiResponse<DashboardStats>>> {
    let mut conn = state.db.get()
        .map_err(|e| AppError::internal(format!("db pool error: {e}")))?;

    let pending_reports: i64 = reports::table
        .filter(reports::status.eq("pending"))
        .count()
        .get_result(&mut conn)
        .map_err(|e| AppError::internal(format!("db error: {e}")))?;

    let active_sanctions: i64 = sanctions::table
        .filter(sanctions::is_active.eq(true))
        .count()
        .get_result(&mut conn)
        .map_err(|e| AppError::internal(format!("db error: {e}")))?;

    let today_start = Utc::now().date_naive().and_hms_opt(0, 0, 0).unwrap();
    let today_start_utc = today_start.and_utc();

    let reports_today: i64 = reports::table
        .filter(reports::created_at.ge(today_start_utc))
        .count()
        .get_result(&mut conn)
        .map_err(|e| AppError::internal(format!("db error: {e}")))?;

    Ok(Json(ApiResponse::ok(DashboardStats {
        pending_reports,
        active_sanctions,
        reports_today,
    })))
}

// --- Audit log (paginated admin actions) ---

pub async fn get_audit_log(
    State(state): State<Arc<AppState>>,
    _admin: AdminUser,
    Query(params): Query<PaginationParams>,
) -> AppResult<Json<ApiResponse<Paginated<AdminAction>>>> {
    let mut conn = state.db.get()
        .map_err(|e| AppError::internal(format!("db pool error: {e}")))?;

    let offset = params.offset() as i64;
    let limit = params.limit() as i64;

    let items = admin_actions::table
        .order(admin_actions::created_at.desc())
        .offset(offset)
        .limit(limit)
        .load::<AdminAction>(&mut conn)
        .map_err(|e| AppError::internal(format!("db error: {e}")))?;

    let total: i64 = admin_actions::table
        .count()
        .get_result(&mut conn)
        .map_err(|e| AppError::internal(format!("db error: {e}")))?;

    let paginated = Paginated::new(items, total as u64, &params);
    Ok(Json(ApiResponse::ok(paginated)))
}

// --- Helper: compute expires_at based on sanction type ---

fn compute_expires_at(sanction_type: &str) -> Option<chrono::DateTime<Utc>> {
    let now = Utc::now();
    match sanction_type {
        "ban_1h" => Some(now + Duration::hours(1)),
        "ban_24h" => Some(now + Duration::hours(24)),
        "ban_30d" => Some(now + Duration::days(30)),
        "ban_permanent" => None,
        "warning" => None,
        _ => None,
    }
}
