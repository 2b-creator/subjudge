
//! Authentication and authorization module.
//!
//! This module provides JWT-based authentication, password hashing, and role extraction
//! for the contest management system. It integrates with the access control system to
//! determine what capabilities and endpoints are available to authenticated users.

use anyhow::{anyhow, Result};
use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
    Json, RequestPartsExt,
};
use axum_extra::{
    headers::{authorization::Bearer, authorization::Basic, Authorization},
    TypedHeader,
};
use bcrypt::{hash, verify, DEFAULT_COST};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};

use crate::models::accounts::{Column, Entity as Accounts};

/// JWT secret key for signing and verifying tokens.
/// In production, this should be loaded from environment variables or secure storage.
const JWT_SECRET: &str = "your-secret-key-change-in-production";

/// JWT token expiration time in seconds (24 hours).
const TOKEN_EXPIRATION_SECONDS: i64 = 86400;

/// User role types for authorization.
///
/// These roles determine what capabilities and endpoints a user has access to
/// in the contest system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UserRole {
    /// Public/unauthenticated access (read-only, basic information).
    Public,
    /// Team participant (can submit solutions, view own data).
    Team,
    /// Administrator/judge (full access to all contest operations).
    Admin,
    /// Judge role (can evaluate submissions, view all data).
    Judge,
}

impl UserRole {
    /// Converts a string account type to a UserRole.
    ///
    /// # Arguments
    ///
    /// * `account_type` - The account type string from the database
    ///
    /// # Returns
    ///
    /// The corresponding UserRole, defaulting to Public for unknown types
    pub fn from_account_type(account_type: &str) -> Self {
        match account_type.to_lowercase().as_str() {
            "admin" | "administrator" => UserRole::Admin,
            "judge" => UserRole::Judge,
            "team" | "participant" => UserRole::Team,
            _ => UserRole::Public,
        }
    }

    /// Checks if this role has admin privileges.
    pub fn is_admin(&self) -> bool {
        matches!(self, UserRole::Admin)
    }

    /// Checks if this role has judge privileges.
    pub fn is_judge(&self) -> bool {
        matches!(self, UserRole::Judge | UserRole::Admin)
    }

    /// Checks if this role is a team participant.
    pub fn is_team(&self) -> bool {
        matches!(self, UserRole::Team)
    }
}

/// JWT claims structure.
///
/// This structure is encoded in the JWT token and contains the authenticated
/// user's identity and role information.
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// Subject (user ID).
    pub sub: String,
    /// Username.
    pub username: String,
    /// User role.
    pub role: UserRole,
    /// Team ID (if the user is associated with a team).
    pub team_id: Option<String>,
    /// Token issued at timestamp (Unix timestamp).
    pub iat: i64,
    /// Token expiration timestamp (Unix timestamp).
    pub exp: i64,
}

impl Claims {
    /// Creates new JWT claims for a user.
    ///
    /// # Arguments
    ///
    /// * `user_id` - The unique user identifier
    /// * `username` - The username
    /// * `role` - The user's role
    /// * `team_id` - Optional team ID if the user is associated with a team
    ///
    /// # Returns
    ///
    /// New Claims with current issued time and calculated expiration
    pub fn new(user_id: String, username: String, role: UserRole, team_id: Option<String>) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            sub: user_id,
            username,
            role,
            team_id,
            iat: now,
            exp: now + TOKEN_EXPIRATION_SECONDS,
        }
    }
}

/// Authenticated user information extracted from JWT token.
///
/// This struct is used as an Axum extractor to automatically validate
/// JWT tokens and extract user information from requests.
#[derive(Debug, Clone)]
pub struct AuthUser {
    /// User ID.
    pub user_id: String,
    /// Username.
    pub username: String,
    /// User role.
    pub role: UserRole,
    /// Team ID (if associated with a team).
    pub team_id: Option<String>,
}

/// Error type for authentication failures.
#[derive(Debug)]
pub struct AuthError {
    message: String,
    status: StatusCode,
}

impl AuthError {
    fn new(message: impl Into<String>, status: StatusCode) -> Self {
        Self {
            message: message.into(),
            status,
        }
    }

    fn unauthorized(message: impl Into<String>) -> Self {
        Self::new(message, StatusCode::UNAUTHORIZED)
    }
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let body = Json(serde_json::json!({
            "error": self.message
        }));
        (self.status, body).into_response()
    }
}

/// Axum extractor for authenticated users.
///
/// Supports multiple authentication methods:
/// 1. HTTP Basic Authentication (RFC 7617) - primary method
/// 2. Bearer token (JWT) - alternative method
///
/// Use this in handler functions to automatically require authentication
/// and extract user information.
///
/// # Example
///
/// ```rust
/// async fn protected_handler(
///     auth_user: AuthUser,
/// ) -> impl IntoResponse {
///     format!("Hello, {}!", auth_user.username)
/// }
/// ```
impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // Try HTTP Basic Authentication first (RFC 7617)
        if let Ok(TypedHeader(Authorization(basic))) =
            parts.extract::<TypedHeader<Authorization<Basic>>>().await
        {
            return Self::from_basic_auth(basic, parts, state).await;
        }

        // Fall back to Bearer token (JWT) authentication
        if let Ok(TypedHeader(Authorization(bearer))) =
            parts.extract::<TypedHeader<Authorization<Bearer>>>().await
        {
            return Self::from_bearer_token(bearer).await;
        }

        Err(AuthError::unauthorized(
            "Missing or invalid authorization header. Use HTTP Basic Auth or Bearer token."
        ))
    }
}

impl AuthUser {
    /// Authenticates using HTTP Basic Authentication.
    async fn from_basic_auth<S>(
        basic: Basic,
        parts: &mut Parts,
        _state: &S,
    ) -> Result<Self, AuthError>
    where
        S: Send + Sync,
    {
        let username = basic.username();
        let password = basic.password();

        // Extract database connection from extensions
        // Note: The DB connection must be added to request extensions by middleware
        let db = parts
            .extensions
            .get::<DatabaseConnection>()
            .ok_or_else(|| AuthError::new(
                "Database connection not available",
                StatusCode::INTERNAL_SERVER_ERROR
            ))?
            .clone();

        // Authenticate the user
        let auth_service = AuthService::new(db);

        // Find user by username
        let user = Accounts::find()
            .filter(Column::AccountUsername.eq(username))
            .one(&auth_service.db)
            .await
            .map_err(|e| AuthError::new(
                format!("Database error: {}", e),
                StatusCode::INTERNAL_SERVER_ERROR
            ))?
            .ok_or_else(|| AuthError::unauthorized("Invalid username or password"))?;

        // Check if account is enabled
        if !user.enabled {
            return Err(AuthError::unauthorized("Account has been disabled"));
        }

        // Verify password
        if let Some(password_hash) = &user.password_hash {
            if !password::verify_password(password, password_hash)
                .map_err(|e| AuthError::new(
                    format!("Password verification error: {}", e),
                    StatusCode::INTERNAL_SERVER_ERROR
                ))?
            {
                return Err(AuthError::unauthorized("Invalid username or password"));
            }
        } else {
            return Err(AuthError::unauthorized("Account has no password set"));
        }

        // Determine user role
        let role = UserRole::from_account_type(&user.account_type);

        Ok(AuthUser {
            user_id: user.id,
            username: user.account_username,
            role,
            team_id: user.team_id,
        })
    }

    /// Authenticates using Bearer token (JWT).
    async fn from_bearer_token(bearer: Bearer) -> Result<Self, AuthError> {
        // Decode and validate the JWT token
        let token_data = decode::<Claims>(
            bearer.token(),
            &DecodingKey::from_secret(JWT_SECRET.as_bytes()),
            &Validation::default(),
        )
        .map_err(|e| AuthError::unauthorized(format!("Invalid token: {}", e)))?;

        let claims = token_data.claims;

        // Check if token is expired
        let now = chrono::Utc::now().timestamp();
        if claims.exp < now {
            return Err(AuthError::unauthorized("Token has expired"));
        }

        Ok(AuthUser {
            user_id: claims.sub,
            username: claims.username,
            role: claims.role,
            team_id: claims.team_id,
        })
    }
}

/// Optional authentication extractor.
///
/// Unlike `AuthUser`, this extractor does not fail if no valid token or credentials are provided.
/// Use this when you want to provide different responses for authenticated vs
/// unauthenticated users. Supports both HTTP Basic Auth and Bearer token.
#[derive(Debug, Clone)]
pub struct OptionalAuthUser(pub Option<AuthUser>);

impl<S> FromRequestParts<S> for OptionalAuthUser
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let auth_user = AuthUser::from_request_parts(parts, state).await.ok();
        Ok(OptionalAuthUser(auth_user))
    }
}

/// Middleware to inject database connection into request extensions.
///
/// This middleware is required for HTTP Basic Authentication to work,
/// as it needs database access to verify credentials on every request.
pub async fn inject_db_middleware(
    axum::extract::State(db): axum::extract::State<DatabaseConnection>,
    mut request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    request.extensions_mut().insert(db);
    next.run(request).await
}

/// Password hashing utilities.
pub mod password {
    use super::*;

    /// Hashes a plaintext password using bcrypt.
    ///
    /// # Arguments
    ///
    /// * `password` - The plaintext password to hash
    ///
    /// # Returns
    ///
    /// Result containing the hashed password string
    pub fn hash_password(password: &str) -> Result<String> {
        hash(password, DEFAULT_COST).map_err(|e| anyhow!("Failed to hash password: {}", e))
    }

    /// Verifies a plaintext password against a bcrypt hash.
    ///
    /// # Arguments
    ///
    /// * `password` - The plaintext password to verify
    /// * `hash` - The bcrypt hash to verify against
    ///
    /// # Returns
    ///
    /// Result indicating whether the password matches
    pub fn verify_password(password: &str, hash: &str) -> Result<bool> {
        verify(password, hash).map_err(|e| anyhow!("Failed to verify password: {}", e))
    }
}

/// JWT token utilities.
pub mod token {
    use super::*;

    /// Generates a JWT token for a user.
    ///
    /// # Arguments
    ///
    /// * `user_id` - The unique user identifier
    /// * `username` - The username
    /// * `role` - The user's role
    /// * `team_id` - Optional team ID
    ///
    /// # Returns
    ///
    /// Result containing the encoded JWT token string
    pub fn generate_token(
        user_id: String,
        username: String,
        role: UserRole,
        team_id: Option<String>,
    ) -> Result<String> {
        let claims = Claims::new(user_id, username, role, team_id);
        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(JWT_SECRET.as_bytes()),
        )
        .map_err(|e| anyhow!("Failed to generate token: {}", e))
    }

    /// Validates and decodes a JWT token.
    ///
    /// # Arguments
    ///
    /// * `token` - The JWT token string to validate
    ///
    /// # Returns
    ///
    /// Result containing the decoded Claims
    pub fn validate_token(token: &str) -> Result<Claims> {
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(JWT_SECRET.as_bytes()),
            &Validation::default(),
        )
        .map_err(|e| anyhow!("Failed to validate token: {}", e))?;

        Ok(token_data.claims)
    }
}

/// Authentication service for user login and verification.
pub struct AuthService {
    db: DatabaseConnection,
}

impl AuthService {
    /// Creates a new authentication service.
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Authenticates a user with username and password.
    ///
    /// # Arguments
    ///
    /// * `username` - The username
    /// * `password` - The plaintext password
    ///
    /// # Returns
    ///
    /// Result containing a JWT token if authentication succeeds
    pub async fn authenticate(&self, username: &str, password: &str) -> Result<String> {
        // Find user by username
        let user = Accounts::find()
            .filter(Column::AccountUsername.eq(username))
            .one(&self.db)
            .await
            .map_err(|e| anyhow!("Database error: {}", e))?
            .ok_or_else(|| anyhow!("Invalid username or password"))?;

        // Check if account is enabled
        if !user.enabled {
            return Err(anyhow!("Account has been disabled"));
        }

        // Verify password
        if let Some(password_hash) = &user.password_hash {
            if !password::verify_password(password, password_hash)? {
                return Err(anyhow!("Invalid username or password"));
            }
        } else {
            return Err(anyhow!("Account has no password set"));
        }

        // Determine user role
        let role = UserRole::from_account_type(&user.account_type);

        // Generate JWT token
        token::generate_token(user.id, user.account_username, role, user.team_id)
    }

    /// Gets user information by user ID.
    ///
    /// # Arguments
    ///
    /// * `user_id` - The user ID
    ///
    /// # Returns
    ///
    /// Result containing the user's role and team ID
    pub async fn get_user_info(&self, user_id: &str) -> Result<(UserRole, Option<String>)> {
        let user = Accounts::find_by_id(user_id)
            .one(&self.db)
            .await
            .map_err(|e| anyhow!("Database error: {}", e))?
            .ok_or_else(|| anyhow!("User not found"))?;

        let role = UserRole::from_account_type(&user.account_type);
        Ok((role, user.team_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_role_from_account_type() {
        assert_eq!(UserRole::from_account_type("admin"), UserRole::Admin);
        assert_eq!(UserRole::from_account_type("ADMIN"), UserRole::Admin);
        assert_eq!(UserRole::from_account_type("judge"), UserRole::Judge);
        assert_eq!(UserRole::from_account_type("team"), UserRole::Team);
        assert_eq!(UserRole::from_account_type("unknown"), UserRole::Public);
    }

    #[test]
    fn test_user_role_privileges() {
        assert!(UserRole::Admin.is_admin());
        assert!(UserRole::Admin.is_judge());
        assert!(!UserRole::Admin.is_team());

        assert!(!UserRole::Judge.is_admin());
        assert!(UserRole::Judge.is_judge());
        assert!(!UserRole::Judge.is_team());

        assert!(!UserRole::Team.is_admin());
        assert!(!UserRole::Team.is_judge());
        assert!(UserRole::Team.is_team());
    }

    #[test]
    fn test_claims_creation() {
        let claims = Claims::new(
            "user123".to_string(),
            "testuser".to_string(),
            UserRole::Team,
            Some("team456".to_string()),
        );

        assert_eq!(claims.sub, "user123");
        assert_eq!(claims.username, "testuser");
        assert_eq!(claims.role, UserRole::Team);
        assert_eq!(claims.team_id, Some("team456".to_string()));
        assert!(claims.exp > claims.iat);
    }

    #[test]
    fn test_password_hashing() {
        let password = "secure_password_123";
        let hash = password::hash_password(password).unwrap();

        assert_ne!(hash, password);
        assert!(password::verify_password(password, &hash).unwrap());
        assert!(!password::verify_password("wrong_password", &hash).unwrap());
    }

    #[test]
    fn test_token_generation_and_validation() {
        let token = token::generate_token(
            "user123".to_string(),
            "testuser".to_string(),
            UserRole::Admin,
            None,
        )
        .unwrap();

        let claims = token::validate_token(&token).unwrap();
        assert_eq!(claims.sub, "user123");
        assert_eq!(claims.username, "testuser");
        assert_eq!(claims.role, UserRole::Admin);
        assert_eq!(claims.team_id, None);
    }
}
