use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

// ─── JSON envelope ──────────────────────────────────────────────────────────

#[derive(Serialize)]
struct ErrorEnvelope {
    error: ErrorBody,
}

#[derive(Serialize)]
struct ErrorBody {
    code: &'static str,
    message: String,
    status: u16,
}

// ─── ApiError ───────────────────────────────────────────────────────────────

/// Structured API error that serializes to JSON.
///
/// ```json
/// {
///   "error": {
///     "code": "room_not_found",
///     "message": "Room 'abc123' does not exist.",
///     "status": 404
///   }
/// }
/// ```
pub struct ApiError {
    pub code: &'static str,
    pub message: String,
    pub status: StatusCode,
}

// ─── IntoResponse ───────────────────────────────────────────────────────────

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        // Log according to severity.
        if self.status.is_server_error() {
            tracing::error!(
                code = self.code,
                status = self.status.as_u16(),
                "{}",
                self.message
            );
        } else if self.status.is_client_error() {
            tracing::warn!(
                code = self.code,
                status = self.status.as_u16(),
                "{}",
                self.message
            );
        }

        let envelope = ErrorEnvelope {
            error: ErrorBody {
                code: self.code,
                message: self.message,
                status: self.status.as_u16(),
            },
        };

        (self.status, Json(envelope)).into_response()
    }
}

// ─── From<StatusCode> (retro-compatibility) ─────────────────────────────────

impl From<StatusCode> for ApiError {
    fn from(status: StatusCode) -> Self {
        let code: &'static str = match status {
            StatusCode::BAD_REQUEST => "bad_request",
            StatusCode::UNAUTHORIZED => "unauthorized",
            StatusCode::FORBIDDEN => "forbidden",
            StatusCode::NOT_FOUND => "not_found",
            StatusCode::CONFLICT => "conflict",
            StatusCode::INTERNAL_SERVER_ERROR => "internal_server_error",
            _ => "unknown_error",
        };

        let message = status
            .canonical_reason()
            .unwrap_or("Unknown error")
            .to_string();

        Self {
            code,
            message,
            status,
        }
    }
}

// ─── Generic constructors ───────────────────────────────────────────────────

impl ApiError {
    /// 401 Unauthorized with a custom message.
    pub fn unauthorized(msg: impl Into<String>) -> Self {
        Self {
            code: "unauthorized",
            message: msg.into(),
            status: StatusCode::UNAUTHORIZED,
        }
    }

    /// 403 Forbidden with a custom message.
    pub fn forbidden(msg: impl Into<String>) -> Self {
        Self {
            code: "forbidden",
            message: msg.into(),
            status: StatusCode::FORBIDDEN,
        }
    }

    /// 404 Not Found with a custom message.
    pub fn not_found(msg: impl Into<String>) -> Self {
        Self {
            code: "not_found",
            message: msg.into(),
            status: StatusCode::NOT_FOUND,
        }
    }

    /// 409 Conflict with a custom message.
    pub fn conflict(msg: impl Into<String>) -> Self {
        Self {
            code: "conflict",
            message: msg.into(),
            status: StatusCode::CONFLICT,
        }
    }

    /// 400 Bad Request with a custom message.
    pub fn bad_request(msg: impl Into<String>) -> Self {
        Self {
            code: "bad_request",
            message: msg.into(),
            status: StatusCode::BAD_REQUEST,
        }
    }

    /// 500 Internal Server Error with a custom message.
    pub fn internal(msg: impl Into<String>) -> Self {
        Self {
            code: "internal_server_error",
            message: msg.into(),
            status: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    // ─── Domain-specific constructors ───────────────────────────────────

    /// 401 — the `Authorization` header is missing or malformed.
    pub fn auth_header_missing() -> Self {
        Self {
            code: "auth_header_missing",
            message: "Authorization header is missing or malformed.".into(),
            status: StatusCode::UNAUTHORIZED,
        }
    }

    /// 401 — the API key is not recognized.
    pub fn api_key_invalid() -> Self {
        Self {
            code: "api_key_invalid",
            message: "The provided API key is not valid.".into(),
            status: StatusCode::UNAUTHORIZED,
        }
    }

    /// 401 — the JWT token is invalid (bad signature, malformed, etc.).
    pub fn token_invalid() -> Self {
        Self {
            code: "token_invalid",
            message: "The provided token is invalid.".into(),
            status: StatusCode::UNAUTHORIZED,
        }
    }

    /// 401 — the JWT token has expired.
    pub fn token_expired() -> Self {
        Self {
            code: "token_expired",
            message: "The provided token has expired.".into(),
            status: StatusCode::UNAUTHORIZED,
        }
    }

    /// 403 — the peer's role does not permit this operation.
    pub fn role_insufficient(role: &str) -> Self {
        Self {
            code: "role_insufficient",
            message: format!("Role '{role}' does not have permission for this operation."),
            status: StatusCode::FORBIDDEN,
        }
    }

    /// 404 — the requested room does not exist.
    pub fn room_not_found(room_id: &str) -> Self {
        Self {
            code: "room_not_found",
            message: format!("Room '{room_id}' does not exist."),
            status: StatusCode::NOT_FOUND,
        }
    }

    /// 409 — the room has reached its maximum capacity.
    pub fn room_full(room_id: &str) -> Self {
        Self {
            code: "room_full",
            message: format!("Room '{room_id}' is full."),
            status: StatusCode::CONFLICT,
        }
    }

    /// 404 — no publisher is available in the requested room.
    pub fn no_publisher(room_id: &str) -> Self {
        Self {
            code: "no_publisher_available",
            message: format!("No publisher is available in room '{room_id}'."),
            status: StatusCode::NOT_FOUND,
        }
    }

    /// 400 — the provided role string is not recognized.
    pub fn invalid_role(role: &str) -> Self {
        Self {
            code: "invalid_role",
            message: format!("Role '{role}' is not a valid role."),
            status: StatusCode::BAD_REQUEST,
        }
    }

    /// 400 — the operation targets a room whose type does not match.
    pub fn room_type_mismatch(expected: &str, actual: &str) -> Self {
        Self {
            code: "room_type_mismatch",
            message: format!(
                "Expected room type '{expected}' but the room is of type '{actual}'."
            ),
            status: StatusCode::BAD_REQUEST,
        }
    }

    /// 500 — the WebRTC peer connection could not be established.
    pub fn peer_connection_failed() -> Self {
        Self {
            code: "peer_connection_failed",
            message: "Failed to establish the WebRTC peer connection.".into(),
            status: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// 400 — the SDP offer/answer is invalid or could not be parsed.
    pub fn invalid_sdp() -> Self {
        Self {
            code: "invalid_sdp",
            message: "The provided SDP is invalid or could not be parsed.".into(),
            status: StatusCode::BAD_REQUEST,
        }
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use axum::response::IntoResponse;

    /// Helper: convert an `ApiError` into its JSON body string.
    async fn body_string(err: ApiError) -> String {
        let response = err.into_response();
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    #[tokio::test]
    async fn json_structure() {
        let json = body_string(ApiError::room_not_found("abc123")).await;
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(value["error"]["code"], "room_not_found");
        assert_eq!(value["error"]["message"], "Room 'abc123' does not exist.");
        assert_eq!(value["error"]["status"], 404);
    }

    #[tokio::test]
    async fn status_code_is_set() {
        let response = ApiError::unauthorized("nope").into_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn from_status_code() {
        let err = ApiError::from(StatusCode::CONFLICT);
        assert_eq!(err.code, "conflict");
        assert_eq!(err.status, StatusCode::CONFLICT);
        assert_eq!(err.message, "Conflict");
    }

    #[tokio::test]
    async fn room_type_mismatch_message() {
        let json = body_string(ApiError::room_type_mismatch("broadcast", "call")).await;
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(value["error"]["code"], "room_type_mismatch");
        assert!(value["error"]["message"]
            .as_str()
            .unwrap()
            .contains("broadcast"));
        assert!(value["error"]["message"]
            .as_str()
            .unwrap()
            .contains("call"));
        assert_eq!(value["error"]["status"], 400);
    }

    #[tokio::test]
    async fn internal_error_500() {
        let response = ApiError::peer_connection_failed().into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let json = body_string(ApiError::peer_connection_failed()).await;
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["error"]["code"], "peer_connection_failed");
        assert_eq!(value["error"]["status"], 500);
    }
}
