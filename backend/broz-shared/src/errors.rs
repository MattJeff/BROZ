use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::types::ApiErrorResponse;

/// Application error codes following the pattern E{service}{sequence}
///
/// Ranges:
/// - E0xxx: Shared/infrastructure errors
/// - E1xxx: Auth errors
/// - E2xxx: User errors
/// - E3xxx: Matching errors
/// - E4xxx: Messaging errors
/// - E5xxx: Notification errors
/// - E6xxx: Moderation errors
/// - E7xxx: Analytics errors
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorCode {
    // Shared (E0xxx)
    InternalError,
    ValidationError,
    NotFound,
    Unauthorized,
    Forbidden,
    RateLimited,
    ServiceUnavailable,
    BadRequest,
    PayloadTooLarge,

    // Auth (E1xxx)
    InvalidCredentials,
    EmailAlreadyExists,
    EmailNotVerified,
    TokenExpired,
    TokenInvalid,
    RefreshTokenRevoked,
    OAuthError,
    PasswordTooWeak,
    VerificationCodeExpired,
    VerificationCodeInvalid,
    ResetCodeExpired,
    ResetCodeInvalid,
    EmailRateLimited,
    UserBanned,

    // User (E2xxx)
    ProfileNotFound,
    DisplayNameTaken,
    InvalidDisplayName,
    PhotoUploadFailed,
    FollowAlreadyExists,
    FollowNotFound,
    CannotFollowSelf,
    OnboardingIncomplete,

    // Matching (E3xxx)
    AlreadyInQueue,
    NotInQueue,
    NotInMatch,
    MatchNotFound,
    LiveCamRequestNotFound,
    LiveCamRequestExpired,
    AlreadyInMatch,

    // Messaging (E4xxx)
    ConversationNotFound,
    NotConversationMember,
    MessageNotFound,
    GroupNameRequired,

    // Notification (E5xxx)
    NotificationNotFound,

    // Moderation (E6xxx)
    ReportNotFound,
    SanctionNotFound,
    ReportAlreadyReviewed,
    CannotReportSelf,
    DuplicateReport,
}

impl ErrorCode {
    pub fn code(&self) -> &'static str {
        match self {
            // Shared
            Self::InternalError => "E0001",
            Self::ValidationError => "E0002",
            Self::NotFound => "E0003",
            Self::Unauthorized => "E0004",
            Self::Forbidden => "E0005",
            Self::RateLimited => "E0006",
            Self::ServiceUnavailable => "E0007",
            Self::BadRequest => "E0008",
            Self::PayloadTooLarge => "E0009",

            // Auth
            Self::InvalidCredentials => "E1001",
            Self::EmailAlreadyExists => "E1002",
            Self::EmailNotVerified => "E1003",
            Self::TokenExpired => "E1004",
            Self::TokenInvalid => "E1005",
            Self::RefreshTokenRevoked => "E1006",
            Self::OAuthError => "E1007",
            Self::PasswordTooWeak => "E1008",
            Self::VerificationCodeExpired => "E1009",
            Self::VerificationCodeInvalid => "E1010",
            Self::ResetCodeExpired => "E1011",
            Self::ResetCodeInvalid => "E1012",
            Self::EmailRateLimited => "E1013",
            Self::UserBanned => "E1014",

            // User
            Self::ProfileNotFound => "E2001",
            Self::DisplayNameTaken => "E2002",
            Self::InvalidDisplayName => "E2003",
            Self::PhotoUploadFailed => "E2004",
            Self::FollowAlreadyExists => "E2005",
            Self::FollowNotFound => "E2006",
            Self::CannotFollowSelf => "E2007",
            Self::OnboardingIncomplete => "E2008",

            // Matching
            Self::AlreadyInQueue => "E3001",
            Self::NotInQueue => "E3002",
            Self::NotInMatch => "E3003",
            Self::MatchNotFound => "E3004",
            Self::LiveCamRequestNotFound => "E3005",
            Self::LiveCamRequestExpired => "E3006",
            Self::AlreadyInMatch => "E3007",

            // Messaging
            Self::ConversationNotFound => "E4001",
            Self::NotConversationMember => "E4002",
            Self::MessageNotFound => "E4003",
            Self::GroupNameRequired => "E4004",

            // Notification
            Self::NotificationNotFound => "E5001",

            // Moderation
            Self::ReportNotFound => "E6001",
            Self::SanctionNotFound => "E6002",
            Self::ReportAlreadyReviewed => "E6003",
            Self::CannotReportSelf => "E6004",
            Self::DuplicateReport => "E6005",
        }
    }

    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::InternalError | Self::ServiceUnavailable => StatusCode::INTERNAL_SERVER_ERROR,
            Self::ValidationError | Self::BadRequest | Self::PasswordTooWeak
            | Self::InvalidDisplayName | Self::GroupNameRequired => StatusCode::BAD_REQUEST,
            Self::PayloadTooLarge => StatusCode::PAYLOAD_TOO_LARGE,
            Self::NotFound | Self::ProfileNotFound | Self::FollowNotFound
            | Self::MatchNotFound | Self::ConversationNotFound | Self::MessageNotFound
            | Self::NotificationNotFound | Self::ReportNotFound | Self::SanctionNotFound
            | Self::LiveCamRequestNotFound | Self::NotInQueue | Self::NotInMatch => StatusCode::NOT_FOUND,
            Self::Unauthorized | Self::InvalidCredentials | Self::TokenExpired
            | Self::TokenInvalid | Self::RefreshTokenRevoked | Self::EmailNotVerified
            | Self::VerificationCodeExpired | Self::VerificationCodeInvalid
            | Self::ResetCodeExpired | Self::ResetCodeInvalid => StatusCode::UNAUTHORIZED,
            Self::Forbidden | Self::UserBanned | Self::OnboardingIncomplete => StatusCode::FORBIDDEN,
            Self::RateLimited | Self::EmailRateLimited => StatusCode::TOO_MANY_REQUESTS,
            Self::EmailAlreadyExists | Self::DisplayNameTaken | Self::FollowAlreadyExists
            | Self::AlreadyInQueue | Self::AlreadyInMatch | Self::ReportAlreadyReviewed
            | Self::DuplicateReport => StatusCode::CONFLICT,
            Self::OAuthError | Self::PhotoUploadFailed | Self::LiveCamRequestExpired => StatusCode::BAD_REQUEST,
            Self::CannotFollowSelf | Self::CannotReportSelf | Self::NotConversationMember => StatusCode::FORBIDDEN,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{message}")]
    Known {
        code: ErrorCode,
        message: String,
        details: Option<serde_json::Value>,
    },

    #[error("internal server error")]
    Internal(#[from] anyhow::Error),

    #[error("database error: {0}")]
    Database(#[from] diesel::result::Error),

    #[error("validation error: {0}")]
    Validation(String),
}

impl AppError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self::Known {
            code,
            message: message.into(),
            details: None,
        }
    }

    pub fn with_details(code: ErrorCode, message: impl Into<String>, details: serde_json::Value) -> Self {
        Self::Known {
            code,
            message: message.into(),
            details: Some(details),
        }
    }

    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::Unauthorized, message)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::NotFound, message)
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::BadRequest, message)
    }

    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::Forbidden, message)
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::InternalError, message)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_response) = match &self {
            AppError::Known { code, message, details } => {
                let status = code.status_code();
                let mut resp = ApiErrorResponse::new(code.code(), message);
                if let Some(d) = details {
                    resp = resp.with_details(d.clone());
                }
                (status, resp)
            }
            AppError::Internal(err) => {
                tracing::error!(error = %err, "internal server error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    ApiErrorResponse::new("E0001", "internal server error"),
                )
            }
            AppError::Database(err) => {
                tracing::error!(error = %err, "database error");
                match err {
                    diesel::result::Error::NotFound => (
                        StatusCode::NOT_FOUND,
                        ApiErrorResponse::new("E0003", "resource not found"),
                    ),
                    _ => (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        ApiErrorResponse::new("E0001", "database error"),
                    ),
                }
            }
            AppError::Validation(msg) => (
                StatusCode::BAD_REQUEST,
                ApiErrorResponse::new("E0002", msg),
            ),
        };

        (status, Json(error_response)).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;
