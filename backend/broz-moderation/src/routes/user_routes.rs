use axum::extract::State;
use axum::Json;
use diesel::prelude::*;
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use broz_shared::errors::{AppError, AppResult, ErrorCode};
use broz_shared::types::auth::AuthUser;
use broz_shared::types::api::ApiResponse;

use crate::events::publisher;
use crate::models::{NewReport, Report};
use crate::schema::reports;
use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct CreateReportRequest {
    pub reported_id: Uuid,
    pub report_type: String,
    pub reason: String,
    pub context: Option<String>,
    pub match_session_id: Option<Uuid>,
    pub message_id: Option<Uuid>,
}

pub async fn create_report(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Json(body): Json<CreateReportRequest>,
) -> AppResult<Json<ApiResponse<Report>>> {
    // Cannot report self
    if auth.id == body.reported_id {
        return Err(AppError::new(ErrorCode::CannotReportSelf, "you cannot report yourself"));
    }

    let mut conn = state.db.get()
        .map_err(|e| AppError::internal(format!("db pool error: {e}")))?;

    // Check for duplicate pending report from same reporter against same user
    let existing: i64 = reports::table
        .filter(reports::reporter_id.eq(auth.id))
        .filter(reports::reported_id.eq(body.reported_id))
        .filter(reports::status.eq("pending"))
        .count()
        .get_result(&mut conn)
        .map_err(|e| AppError::internal(format!("db error: {e}")))?;

    if existing > 0 {
        return Err(AppError::new(
            ErrorCode::DuplicateReport,
            "you already have a pending report against this user",
        ));
    }

    let new_report = NewReport {
        reporter_id: auth.id,
        reported_id: body.reported_id,
        report_type: body.report_type.clone(),
        reason: body.reason,
        context: body.context,
        match_session_id: body.match_session_id,
        message_id: body.message_id,
    };

    let report: Report = diesel::insert_into(reports::table)
        .values(&new_report)
        .get_result(&mut conn)
        .map_err(|e| AppError::internal(format!("failed to create report: {e}")))?;

    // Publish event
    publisher::publish_report_created(
        &state.rabbitmq,
        report.id,
        report.reporter_id,
        report.reported_id,
        &report.report_type,
    )
    .await;

    Ok(Json(ApiResponse::ok(report)))
}
