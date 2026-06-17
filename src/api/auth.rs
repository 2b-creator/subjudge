//! Authentication API endpoints.
//!
//! This module provides endpoints for user authentication, including login
//! and token refresh operations.

use crate::auth::{AuthService, AuthUser};
use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};

/// Request body for login endpoint.
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    /// Username for authentication.
    pub username: String,
    /// Password for authentication.
    pub password: String,
}

/// Response body for successful login.
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    /// JWT token for subsequent authenticated requests.
    pub token: String,
    /// Token type (always "Bearer").
    pub token_type: String,
    /// Token expiration time in seconds.
    pub expires_in: i64,
}

/// Error response body.
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    /// Error message.
    pub error: String,
}

/// Authenticates a user and returns a JWT token.
///
/// This endpoint accepts a username and password, verifies the credentials
/// against the database, and returns a JWT token that can be used for
/// subsequent authenticated requests.
///
/// # Request Body
///
/// ```json
/// {
///   "username": "user@example.com",
///   "password": "secure_password"
/// }
/// ```
///
/// # Success Response (200 OK)
///
/// ```json
/// {
///   "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
///   "token_type": "Bearer",
///   "expires_in": 86400
/// }
/// ```
///
/// # Error Responses
///
/// - **401 Unauthorized**: Invalid username or password
/// - **500 Internal Server Error**: Database or server error
///
/// # Usage
///
/// After receiving the token, include it in the Authorization header:
/// ```
/// Authorization: Bearer <token>
/// ```
pub async fn login(
    State(db): State<DatabaseConnection>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Create authentication service
    let auth_service = AuthService::new(db);

    // Authenticate user
    match auth_service.authenticate(&payload.username, &payload.password).await {
        Ok(token) => Ok(Json(LoginResponse {
            token,
            token_type: "Bearer".to_string(),
            expires_in: 86400, // 24 hours
        })),
        Err(e) => {
            let error_msg = e.to_string();
            let status = if error_msg.contains("Invalid username or password") {
                StatusCode::UNAUTHORIZED
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };

            Err((status, Json(ErrorResponse {
                error: error_msg,
            })))
        }
    }
}

/// Gets information about the currently authenticated user.
///
/// This endpoint returns details about the user associated with the
/// provided JWT token. It requires authentication.
///
/// # Authentication
///
/// Required. Include JWT token in Authorization header:
/// ```
/// Authorization: Bearer <token>
/// ```
///
/// # Success Response (200 OK)
///
/// ```json
/// {
///   "user_id": "user123",
///   "username": "user@example.com",
///   "role": "team",
///   "team_id": "team456"
/// }
/// ```
///
/// # Error Responses
///
/// - **401 Unauthorized**: Missing, invalid, or expired token
pub async fn get_current_user(
    auth_user: AuthUser,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "user_id": auth_user.user_id,
        "username": auth_user.username,
        "role": auth_user.role,
        "team_id": auth_user.team_id,
    }))
}

/// Health check endpoint to verify authentication service is available.
///
/// This endpoint can be used to check if the authentication service
/// is operational. It does not require authentication.
///
/// # Success Response (200 OK)
///
/// ```json
/// {
///   "status": "ok",
///   "service": "authentication"
/// }
/// ```
pub async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "service": "authentication"
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_login_request_deserialization() {
        let json = r#"{"username": "test", "password": "pass"}"#;
        let req: LoginRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.username, "test");
        assert_eq!(req.password, "pass");
    }

    #[test]
    fn test_login_response_serialization() {
        let resp = LoginResponse {
            token: "test_token".to_string(),
            token_type: "Bearer".to_string(),
            expires_in: 86400,
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("test_token"));
        assert!(json.contains("Bearer"));
    }
}
