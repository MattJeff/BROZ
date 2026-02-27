use chrono::{DateTime, NaiveDate, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::schema::{profiles, follows, likes};

// --- Profile ---

#[derive(Debug, Queryable, Identifiable, Serialize, Clone)]
#[diesel(table_name = profiles)]
pub struct Profile {
    pub id: Uuid,
    pub credential_id: Uuid,
    pub display_name: Option<String>,
    pub bio: Option<String>,
    pub birth_date: Option<NaiveDate>,
    #[serde(rename = "profile_photo")]
    pub profile_photo_url: Option<String>,
    pub kinks: serde_json::Value,
    pub onboarding_complete: bool,
    pub moderation_status: String,
    pub total_likes: i32,
    pub country: Option<String>,
    pub is_online: bool,
    pub last_seen_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = profiles)]
pub struct NewProfile {
    pub credential_id: Uuid,
}

#[derive(Debug, AsChangeset, Deserialize, Default)]
#[diesel(table_name = profiles)]
pub struct UpdateProfile {
    pub display_name: Option<String>,
    pub bio: Option<String>,
    pub birth_date: Option<NaiveDate>,
    pub profile_photo_url: Option<String>,
    pub kinks: Option<serde_json::Value>,
    pub country: Option<String>,
    pub onboarding_complete: Option<bool>,
}

// --- Follow ---

#[derive(Debug, Queryable, Identifiable, Serialize)]
#[diesel(table_name = follows)]
pub struct Follow {
    pub id: Uuid,
    pub follower_id: Uuid,
    pub following_id: Uuid,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = follows)]
pub struct NewFollow {
    pub follower_id: Uuid,
    pub following_id: Uuid,
}

// --- Like ---

#[derive(Debug, Queryable, Identifiable, Serialize)]
#[diesel(table_name = likes)]
pub struct Like {
    pub id: Uuid,
    pub liker_id: Uuid,
    pub liked_id: Uuid,
    pub match_session_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = likes)]
pub struct NewLike {
    pub liker_id: Uuid,
    pub liked_id: Uuid,
    pub match_session_id: Option<Uuid>,
}
