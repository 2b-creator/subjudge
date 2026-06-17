//! Account management API endpoints.
//!
//! This module provides endpoints for managing user accounts, including
//! disabling and enabling accounts without affecting the contest state.

use crate::auth::AuthUser;
use crate::models::accounts::{ActiveModel, Entity as Accounts};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set};
use serde::{Deserialize, Serialize};

/// Request body for disabling/enabling an account.
#[derive(Debug, Deserialize)]
pub struct UpdateAccountStatusRequest {
    /// Whether the account should be enabled (true) or disabled (false).
    pub enabled: bool,
    /// Optional reason for the status change.
    pub reason: Option<String>,
}

/// Response body for account status operations.
#[derive(Debug, Serialize)]
pub struct AccountStatusResponse {
    /// The account ID.
    pub id: String,
    /// The account username.
    pub username: String,
    /// Whether the account is enabled.
    pub enabled: bool,
    /// Success message.
    pub message: String,
}

/// Response body for listing accounts.
#[derive(Debug, Serialize)]
pub struct AccountListResponse {
    /// The account ID.
    pub id: String,
    /// The account username.
    pub username: String,
    /// The account name.
    pub name: String,
    /// The account type (admin, judge, team, etc.).
    #[serde(rename = "type")]
    pub account_type: String,
    /// Whether the account is enabled.
    pub enabled: bool,
    /// Team ID if associated with a team.
    pub team_id: Option<String>,
}

/// Error response body.
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    /// Error message.
    pub error: String,
}

/// Disables a user account.
///
/// This endpoint allows administrators to disable any account (human user or system account)
/// without needing to start or stop the contest. Disabled accounts cannot authenticate
/// or perform any actions in the system.
///
/// # Authentication
///
/// Required. Must be authenticated as an administrator.
///
/// # Path Parameters
///
/// - `account_id`: The unique identifier of the account to disable
///
/// # Request Body
///
/// ```json
/// {
///   "enabled": false,
///   "reason": "Account suspended for policy violation"
/// }
/// ```
///
/// # Success Response (200 OK)
///
/// ```json
/// {
///   "id": "account123",
///   "username": "user@example.com",
///   "enabled": false,
///   "message": "Account disabled successfully"
/// }
/// ```
///
/// # Error Responses
///
/// - **401 Unauthorized**: Not authenticated or insufficient permissions
/// - **403 Forbidden**: User is not an administrator
/// - **404 Not Found**: Account not found
/// - **500 Internal Server Error**: Database or server error
pub async fn update_account_status(
    auth_user: AuthUser,
    State(db): State<DatabaseConnection>,
    Path(account_id): Path<String>,
    Json(payload): Json<UpdateAccountStatusRequest>,
) -> Result<Json<AccountStatusResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Check if user is an administrator
    if !auth_user.role.is_admin() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "Only administrators can modify account status".to_string(),
            }),
        ));
    }

    // Find the account
    let account = Accounts::find_by_id(&account_id)
        .one(&db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Database error: {}", e),
                }),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("Account not found: {}", account_id),
                }),
            )
        })?;

    // Update account status
    let mut account_active: ActiveModel = account.clone().into();
    account_active.enabled = Set(payload.enabled);

    account_active.update(&db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to update account: {}", e),
            }),
        )
    })?;

    // Log the action if reason is provided
    if let Some(reason) = payload.reason {
        eprintln!(
            "Account status changed: account_id={}, username={}, enabled={}, admin={}, reason={}",
            account_id, account.account_username, payload.enabled, auth_user.username, reason
        );
    }

    let action = if payload.enabled {
        "enabled"
    } else {
        "disabled"
    };

    Ok(Json(AccountStatusResponse {
        id: account.id,
        username: account.account_username,
        enabled: payload.enabled,
        message: format!("Account {} successfully", action),
    }))
}

/// Disables a user account (convenience endpoint).
///
/// This is a convenience endpoint that specifically disables an account.
/// Equivalent to calling update_account_status with enabled: false.
///
/// # Authentication
///
/// Required. Must be authenticated as an administrator.
///
/// # Path Parameters
///
/// - `account_id`: The unique identifier of the account to disable
///
/// # Success Response (200 OK)
///
/// ```json
/// {
///   "id": "account123",
///   "username": "user@example.com",
///   "enabled": false,
///   "message": "Account disabled successfully"
/// }
/// ```
pub async fn disable_account(
    auth_user: AuthUser,
    State(db): State<DatabaseConnection>,
    Path(account_id): Path<String>,
) -> Result<Json<AccountStatusResponse>, (StatusCode, Json<ErrorResponse>)> {
    update_account_status(
        auth_user,
        State(db),
        Path(account_id),
        Json(UpdateAccountStatusRequest {
            enabled: false,
            reason: None,
        }),
    )
    .await
}

/// Enables a user account (convenience endpoint).
///
/// This is a convenience endpoint that specifically enables an account.
/// Equivalent to calling update_account_status with enabled: true.
///
/// # Authentication
///
/// Required. Must be authenticated as an administrator.
///
/// # Path Parameters
///
/// - `account_id`: The unique identifier of the account to enable
///
/// # Success Response (200 OK)
///
/// ```json
/// {
///   "id": "account123",
///   "username": "user@example.com",
///   "enabled": true,
///   "message": "Account enabled successfully"
/// }
/// ```
pub async fn enable_account(
    auth_user: AuthUser,
    State(db): State<DatabaseConnection>,
    Path(account_id): Path<String>,
) -> Result<Json<AccountStatusResponse>, (StatusCode, Json<ErrorResponse>)> {
    update_account_status(
        auth_user,
        State(db),
        Path(account_id),
        Json(UpdateAccountStatusRequest {
            enabled: true,
            reason: None,
        }),
    )
    .await
}

/// Lists all accounts with their status.
///
/// This endpoint allows administrators to view all accounts in the system
/// along with their enabled/disabled status.
///
/// # Authentication
///
/// Required. Must be authenticated as an administrator.
///
/// # Success Response (200 OK)
///
/// ```json
/// [
///   {
///     "id": "account123",
///     "username": "user@example.com",
///     "name": "John Doe",
///     "type": "team",
///     "enabled": true,
///     "team_id": "team456"
///   }
/// ]
/// ```
pub async fn list_accounts(
    auth_user: AuthUser,
    State(db): State<DatabaseConnection>,
) -> Result<Json<Vec<AccountListResponse>>, (StatusCode, Json<ErrorResponse>)> {
    // Check if user is an administrator
    if !auth_user.role.is_admin() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "Only administrators can list accounts".to_string(),
            }),
        ));
    }

    // Fetch all accounts
    let accounts = Accounts::find().all(&db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Database error: {}", e),
            }),
        )
    })?;

    let response: Vec<AccountListResponse> = accounts
        .into_iter()
        .map(|account| AccountListResponse {
            id: account.id,
            username: account.account_username,
            name: account.name,
            account_type: account.account_type,
            enabled: account.enabled,
            team_id: account.team_id,
        })
        .collect();

    Ok(Json(response))
}

/// Gets the status of a specific account.
///
/// # Authentication
///
/// Required. Must be authenticated as an administrator.
///
/// # Path Parameters
///
/// - `account_id`: The unique identifier of the account
///
/// # Success Response (200 OK)
///
/// ```json
/// {
///   "id": "account123",
///   "username": "user@example.com",
///   "name": "John Doe",
///   "type": "team",
///   "enabled": true,
///   "team_id": "team456"
/// }
/// ```
pub async fn get_account_status(
    auth_user: AuthUser,
    State(db): State<DatabaseConnection>,
    Path(account_id): Path<String>,
) -> Result<Json<AccountListResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Check if user is an administrator
    if !auth_user.role.is_admin() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "Only administrators can view account details".to_string(),
            }),
        ));
    }

    // Find the account
    let account = Accounts::find_by_id(&account_id)
        .one(&db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Database error: {}", e),
                }),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("Account not found: {}", account_id),
                }),
            )
        })?;

    Ok(Json(AccountListResponse {
        id: account.id,
        username: account.account_username,
        name: account.name,
        account_type: account.account_type,
        enabled: account.enabled,
        team_id: account.team_id,
    }))
}

/// DOC todo

// todo
// pub async fn change_account_password(
//     auth_user: AuthUser,
//     State(db): State<DatabaseConnection>,
//     Path(account_id): Path<String>,
// ) -> Result<Json<AccountListResponse>, (StatusCode, Json<ErrorResponse>)> {
//     if !auth_user.role.is_admin() {
//         return Err((
//             StatusCode::FORBIDDEN,
//             Json(ErrorResponse {
//                 error: "Only administrators can change account password.".to_string(),
//             }),
//         ));
//     }

//     let account = Accounts::find_by_id(&account_id)
//         .one(&db)
//         .await
//         .map_err(|e| {
//             (
//                 StatusCode::INTERNAL_SERVER_ERROR,
//                 Json(ErrorResponse {
//                     error: format!("Database error: {}", e),
//                 }),
//             )
//         })?
//         .ok_or_else(|| {
//             (
//                 StatusCode::NOT_FOUND,
//                 Json(ErrorResponse {
//                     error: format!("Account not found: {}", account_id),
//                 }),
//             )
//         })?;

// }
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_account_status_request_deserialization() {
        let json = r#"{"enabled": false, "reason": "Policy violation"}"#;
        let req: UpdateAccountStatusRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.enabled, false);
        assert_eq!(req.reason, Some("Policy violation".to_string()));
    }

    #[test]
    fn test_account_status_response_serialization() {
        let resp = AccountStatusResponse {
            id: "test123".to_string(),
            username: "testuser".to_string(),
            enabled: false,
            message: "Account disabled successfully".to_string(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("test123"));
        assert!(json.contains("testuser"));
        assert!(json.contains("false"));
    }
}
