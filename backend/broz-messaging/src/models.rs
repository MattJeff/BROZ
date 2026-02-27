use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::schema::{conversations, conversation_members, messages};

// --- Conversation ---

#[derive(Debug, Queryable, Identifiable, Serialize, Clone)]
#[diesel(table_name = conversations)]
pub struct Conversation {
    pub id: Uuid,
    pub is_group: bool,
    pub group_name: Option<String>,
    pub group_photo_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = conversations)]
pub struct NewConversation {
    pub is_group: bool,
    pub group_name: Option<String>,
    pub group_photo_url: Option<String>,
}

// --- ConversationMember ---

#[derive(Debug, Queryable, Identifiable, Serialize, Clone)]
#[diesel(table_name = conversation_members)]
pub struct ConversationMember {
    pub id: Uuid,
    pub conversation_id: Uuid,
    pub user_id: Uuid,
    pub joined_at: DateTime<Utc>,
    pub last_read_at: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = conversation_members)]
pub struct NewConversationMember {
    pub conversation_id: Uuid,
    pub user_id: Uuid,
}

// --- Message ---

#[derive(Debug, Queryable, Identifiable, Serialize, Clone)]
#[diesel(table_name = messages)]
pub struct Message {
    pub id: Uuid,
    pub conversation_id: Uuid,
    pub sender_id: Uuid,
    pub content: Option<String>,
    pub media_url: Option<String>,
    pub media_type: Option<String>,
    pub is_deleted: bool,
    pub is_private: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = messages)]
pub struct NewMessage {
    pub conversation_id: Uuid,
    pub sender_id: Uuid,
    pub content: Option<String>,
    pub media_url: Option<String>,
    pub media_type: Option<String>,
    pub is_private: bool,
}
