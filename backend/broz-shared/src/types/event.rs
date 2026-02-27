use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// RabbitMQ Event envelope wrapping all domain events.
///
/// Routing key format: `broz.{domain}.{entity}.{action}`
/// Example: `broz.auth.user.registered`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event<T: Serialize> {
    pub id: Uuid,
    pub source: String,
    pub event_type: String,
    pub timestamp: DateTime<Utc>,
    pub correlation_id: Option<Uuid>,
    pub user_id: Option<Uuid>,
    pub data: T,
}

impl<T: Serialize> Event<T> {
    pub fn new(source: impl Into<String>, event_type: impl Into<String>, data: T) -> Self {
        Self {
            id: Uuid::now_v7(),
            source: source.into(),
            event_type: event_type.into(),
            timestamp: Utc::now(),
            correlation_id: None,
            user_id: None,
            data,
        }
    }

    pub fn with_user(mut self, user_id: Uuid) -> Self {
        self.user_id = Some(user_id);
        self
    }

    pub fn with_correlation(mut self, correlation_id: Uuid) -> Self {
        self.correlation_id = Some(correlation_id);
        self
    }
}

/// RabbitMQ routing keys
pub mod routing_keys {
    // Auth events
    pub const AUTH_USER_REGISTERED: &str = "broz.auth.user.registered";
    pub const AUTH_USER_BANNED: &str = "broz.auth.user.banned";

    // User events
    pub const USER_PROFILE_UPDATED: &str = "broz.user.profile.updated";
    pub const USER_ONBOARDING_COMPLETED: &str = "broz.user.profile.onboarding_completed";
    pub const USER_FOLLOW_REQUESTED: &str = "broz.user.follow.requested";
    pub const USER_FOLLOW_ACCEPTED: &str = "broz.user.follow.accepted";
    pub const USER_FOLLOW_REMOVED: &str = "broz.user.follow.removed";
    pub const USER_LIKE_SENT: &str = "broz.user.like.sent";

    // Matching events
    pub const MATCHING_SESSION_STARTED: &str = "broz.matching.session.started";
    pub const MATCHING_SESSION_ENDED: &str = "broz.matching.session.ended";
    pub const MATCHING_LIVECAM_REQUESTED: &str = "broz.matching.livecam.requested";
    pub const MATCHING_LIVECAM_RESPONDED: &str = "broz.matching.livecam.responded";

    // Messaging events
    pub const MESSAGING_MESSAGE_SENT: &str = "broz.messaging.message.sent";

    // Moderation events
    pub const MODERATION_REPORT_CREATED: &str = "broz.moderation.report.created";
    pub const MODERATION_SANCTION_ISSUED: &str = "broz.moderation.sanction.issued";
    pub const MODERATION_SANCTION_LIFTED: &str = "broz.moderation.sanction.lifted";
}

/// Common event data payloads
pub mod payloads {
    use serde::{Deserialize, Serialize};
    use uuid::Uuid;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct UserRegistered {
        pub credential_id: Uuid,
        pub email: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct UserBanned {
        pub credential_id: Uuid,
        pub is_banned: bool,
        pub ban_until: Option<chrono::DateTime<chrono::Utc>>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ProfileUpdated {
        pub profile_id: Uuid,
        pub credential_id: Uuid,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct OnboardingCompleted {
        pub credential_id: Uuid,
        pub display_name: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct FollowRequested {
        pub follower_id: Uuid,
        pub following_id: Uuid,
        pub follower_display_name: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct FollowAccepted {
        pub follower_id: Uuid,
        pub following_id: Uuid,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct FollowRemoved {
        pub follower_id: Uuid,
        pub following_id: Uuid,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct LikeSent {
        pub liker_id: Uuid,
        pub liked_id: Uuid,
        pub liker_display_name: String,
        pub match_session_id: Option<Uuid>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct MatchSessionStarted {
        pub match_id: Uuid,
        pub user_a_id: Uuid,
        pub user_b_id: Uuid,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct MatchSessionEnded {
        pub match_id: Uuid,
        pub user_a_id: Uuid,
        pub user_b_id: Uuid,
        pub duration_secs: i32,
        pub end_reason: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct LiveCamRequested {
        pub request_id: Uuid,
        pub requester_id: Uuid,
        pub target_id: Uuid,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct LiveCamResponded {
        pub request_id: Uuid,
        pub requester_id: Uuid,
        pub target_id: Uuid,
        pub accepted: bool,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct MessageSent {
        pub message_id: Uuid,
        pub conversation_id: Uuid,
        pub sender_id: Uuid,
        pub sender_display_name: String,
        pub content_preview: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ReportCreated {
        pub report_id: Uuid,
        pub reporter_id: Uuid,
        pub reported_id: Uuid,
        pub report_type: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct SanctionIssued {
        pub sanction_id: Uuid,
        pub user_id: Uuid,
        pub sanction_type: String,
        pub reason: String,
        pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct SanctionLifted {
        pub sanction_id: Uuid,
        pub user_id: Uuid,
    }
}
