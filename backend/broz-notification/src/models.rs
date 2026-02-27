use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::Serialize;
use uuid::Uuid;

use crate::schema::notifications;

#[derive(Debug, Queryable, Identifiable, Serialize)]
#[diesel(table_name = notifications)]
pub struct Notification {
    pub id: Uuid,
    pub user_id: Uuid,
    pub notification_type: String,
    pub title: String,
    pub body: String,
    pub data: Option<serde_json::Value>,
    pub is_read: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = notifications)]
pub struct NewNotification {
    pub user_id: Uuid,
    pub notification_type: String,
    pub title: String,
    pub body: String,
    pub data: Option<serde_json::Value>,
}
