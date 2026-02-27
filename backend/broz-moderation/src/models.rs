use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::Serialize;
use uuid::Uuid;

use crate::schema::{reports, sanctions, admin_actions};

// --- Report ---

#[derive(Debug, Queryable, Identifiable, Serialize, Clone)]
#[diesel(table_name = reports)]
pub struct Report {
    pub id: Uuid,
    pub reporter_id: Uuid,
    pub reported_id: Uuid,
    pub report_type: String,
    pub reason: String,
    pub context: Option<String>,
    pub match_session_id: Option<Uuid>,
    pub message_id: Option<Uuid>,
    pub status: String,
    pub reviewed_by: Option<Uuid>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = reports)]
pub struct NewReport {
    pub reporter_id: Uuid,
    pub reported_id: Uuid,
    pub report_type: String,
    pub reason: String,
    pub context: Option<String>,
    pub match_session_id: Option<Uuid>,
    pub message_id: Option<Uuid>,
}

// --- Sanction ---

#[derive(Debug, Queryable, Identifiable, Serialize, Clone)]
#[diesel(table_name = sanctions)]
pub struct Sanction {
    pub id: Uuid,
    pub user_id: Uuid,
    pub report_id: Option<Uuid>,
    pub sanction_type: String,
    pub reason: String,
    pub issued_by: Uuid,
    pub expires_at: Option<DateTime<Utc>>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = sanctions)]
pub struct NewSanction {
    pub user_id: Uuid,
    pub report_id: Option<Uuid>,
    pub sanction_type: String,
    pub reason: String,
    pub issued_by: Uuid,
    pub expires_at: Option<DateTime<Utc>>,
}

// --- AdminAction ---

#[derive(Debug, Queryable, Identifiable, Serialize, Clone)]
#[diesel(table_name = admin_actions)]
pub struct AdminAction {
    pub id: Uuid,
    pub admin_id: Uuid,
    pub action: String,
    pub target_user_id: Option<Uuid>,
    pub details: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = admin_actions)]
pub struct NewAdminAction {
    pub admin_id: Uuid,
    pub action: String,
    pub target_user_id: Option<Uuid>,
    pub details: Option<serde_json::Value>,
}
