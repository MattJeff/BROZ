use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::schema::{match_sessions, livecam_requests};

// --- MatchSession ---

#[derive(Debug, Queryable, Identifiable, Serialize, Clone)]
#[diesel(table_name = match_sessions)]
pub struct MatchSession {
    pub id: Uuid,
    pub user_a_id: Uuid,
    pub user_b_id: Uuid,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub end_reason: Option<String>,
    pub duration_secs: Option<i32>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = match_sessions)]
pub struct NewMatchSession {
    pub user_a_id: Uuid,
    pub user_b_id: Uuid,
}

// --- LiveCamRequest ---

#[derive(Debug, Queryable, Identifiable, Serialize, Clone)]
#[diesel(table_name = livecam_requests)]
pub struct LiveCamRequest {
    pub id: Uuid,
    pub requester_id: Uuid,
    pub target_id: Uuid,
    pub status: String,
    pub room_id: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub responded_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Insertable, Deserialize)]
#[diesel(table_name = livecam_requests)]
pub struct NewLiveCamRequest {
    pub requester_id: Uuid,
    pub target_id: Uuid,
    pub expires_at: DateTime<Utc>,
}
