//! 认证与授权模块。
//!
//! 基于 `axum_auth` 提供 HTTP Basic / Bearer (JWT) 两种认证方式，
//! 并通过 `AuthUser` 提取器在 handler 中自动完成校验。

use anyhow::{Result, anyhow};
use axum::{
    Json,
    extract::FromRequestParts,
    http::{StatusCode, request::Parts},
    response::{IntoResponse, Response},
};
use axum_auth::{AuthBasic, AuthBearer};
use bcrypt::{DEFAULT_COST, hash, verify};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};

use crate::models::accounts::{Column, Entity as Accounts};

/// JWT 签名密钥。生产环境应从环境变量或安全存储加载。
const JWT_SECRET: &str = "your-secret-key-change-in-production";

/// JWT 过期时间（秒），默认 24 小时。
const TOKEN_EXPIRATION_SECONDS: i64 = 86400;

/// 用户角色。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UserRole {
    Public,
    Team,
    Admin,
    Judge,
}

impl UserRole {
    /// 由数据库中的账户类型字符串转换为角色，未知类型默认为 `Public`。
    pub fn from_account_type(account_type: &str) -> Self {
        match account_type.to_lowercase().as_str() {
            "admin" | "administrator" => UserRole::Admin,
            "judge" => UserRole::Judge,
            "team" | "participant" => UserRole::Team,
            _ => UserRole::Public,
        }
    }

    pub fn is_admin(&self) -> bool {
        matches!(self, UserRole::Admin)
    }

    pub fn is_judge(&self) -> bool {
        matches!(self, UserRole::Judge | UserRole::Admin)
    }

    pub fn is_team(&self) -> bool {
        matches!(self, UserRole::Team | UserRole::Admin)
    }
}

/// JWT 载荷。
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub username: String,
    pub role: UserRole,
    pub team_id: Option<String>,
    pub iat: i64,
    pub exp: i64,
}

impl Claims {
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

/// 已认证用户信息，可作为 Axum 提取器直接用在 handler 参数上。
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: String,
    pub username: String,
    pub role: UserRole,
    pub team_id: Option<String>,
}

/// 认证失败错误。
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
        (
            self.status,
            Json(serde_json::json!({ "error": self.message })),
        )
            .into_response()
    }
}

impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // 优先 Basic 认证（需要数据库校验），否则回退到 Bearer (JWT)。
        if let Ok(AuthBasic((username, password))) =
            AuthBasic::from_request_parts(parts, _state).await
        {
            return Self::from_basic(parts, &username, password.as_deref()).await;
        }

        if let Ok(AuthBearer(token)) = AuthBearer::from_request_parts(parts, _state).await {
            return Self::from_bearer(&token);
        }

        Err(AuthError::unauthorized(
            "缺少或无效的 Authorization 头，请使用 HTTP Basic Auth 或 Bearer token。",
        ))
    }
}

impl AuthUser {
    /// 通过 HTTP Basic 凭据查库校验。
    async fn from_basic(
        parts: &Parts,
        username: &str,
        password: Option<&str>,
    ) -> Result<Self, AuthError> {
        let db = parts
            .extensions
            .get::<DatabaseConnection>()
            .ok_or_else(|| AuthError::new("数据库连接不可用", StatusCode::INTERNAL_SERVER_ERROR))?;

        let user = Accounts::find()
            .filter(Column::AccountUsername.eq(username))
            .one(db)
            .await
            .map_err(|e| {
                AuthError::new(
                    format!("数据库错误: {e}"),
                    StatusCode::INTERNAL_SERVER_ERROR,
                )
            })?
            .ok_or_else(|| AuthError::unauthorized("用户名或密码错误"))?;

        if !user.enabled {
            return Err(AuthError::unauthorized("账户已被禁用"));
        }

        let hash = user
            .password_hash
            .as_deref()
            .ok_or_else(|| AuthError::unauthorized("账户未设置密码"))?;
        let password = password.ok_or_else(|| AuthError::unauthorized("缺少密码"))?;
        let valid = password::verify_password(password, hash).map_err(|e| {
            AuthError::new(
                format!("密码校验错误: {e}"),
                StatusCode::INTERNAL_SERVER_ERROR,
            )
        })?;
        if !valid {
            return Err(AuthError::unauthorized("用户名或密码错误"));
        }

        Ok(AuthUser {
            role: UserRole::from_account_type(&user.account_type),
            user_id: user.id,
            username: user.account_username,
            team_id: user.team_id,
        })
    }

    /// 校验 Bearer (JWT) token。
    fn from_bearer(token: &str) -> Result<Self, AuthError> {
        let claims = token::validate_token(token)
            .map_err(|e| AuthError::unauthorized(format!("无效的 token: {e}")))?;

        Ok(AuthUser {
            user_id: claims.sub,
            username: claims.username,
            role: claims.role,
            team_id: claims.team_id,
        })
    }
}

/// 可选认证提取器：无有效凭据时不报错，返回 `None`。
#[derive(Debug, Clone)]
pub struct OptionalAuthUser(pub Option<AuthUser>);

impl<S> FromRequestParts<S> for OptionalAuthUser
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        Ok(OptionalAuthUser(
            AuthUser::from_request_parts(parts, state).await.ok(),
        ))
    }
}

/// 注入数据库连接到请求扩展，供 Basic 认证查库使用。
pub async fn inject_db_middleware(
    axum::extract::State(db): axum::extract::State<DatabaseConnection>,
    mut request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    request.extensions_mut().insert(db);
    next.run(request).await
}

/// 密码哈希工具。
pub mod password {
    use super::*;

    pub fn hash_password(password: &str) -> Result<String> {
        hash(password, DEFAULT_COST).map_err(|e| anyhow!("密码哈希失败: {e}"))
    }

    pub fn verify_password(password: &str, hash: &str) -> Result<bool> {
        verify(password, hash).map_err(|e| anyhow!("密码校验失败: {e}"))
    }
}

/// JWT token 工具。
pub mod token {
    use super::*;

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
        .map_err(|e| anyhow!("生成 token 失败: {e}"))
    }

    pub fn validate_token(token: &str) -> Result<Claims> {
        decode::<Claims>(
            token,
            &DecodingKey::from_secret(JWT_SECRET.as_bytes()),
            &Validation::default(),
        )
        .map(|data| data.claims)
        .map_err(|e| anyhow!("校验 token 失败: {e}"))
    }
}

/// 认证服务：用于登录接口签发 JWT。
pub struct AuthService {
    db: DatabaseConnection,
}

impl AuthService {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// 校验用户名密码，成功则返回 JWT token。
    pub async fn authenticate(&self, username: &str, password: &str) -> Result<String> {
        let user = Accounts::find()
            .filter(Column::AccountUsername.eq(username))
            .one(&self.db)
            .await
            .map_err(|e| anyhow!("数据库错误: {e}"))?
            .ok_or_else(|| anyhow!("Invalid username or password"))?;

        if !user.enabled {
            return Err(anyhow!("Account has been disabled"));
        }

        let hash = user
            .password_hash
            .as_deref()
            .ok_or_else(|| anyhow!("Account has no password set"))?;
        if !password::verify_password(password, hash)? {
            return Err(anyhow!("Invalid username or password"));
        }

        let role = UserRole::from_account_type(&user.account_type);
        token::generate_token(user.id, user.account_username, role, user.team_id)
    }

    /// 按用户 ID 获取角色与 team_id。
    pub async fn get_user_info(&self, user_id: &str) -> Result<(UserRole, Option<String>)> {
        let user = Accounts::find_by_id(user_id)
            .one(&self.db)
            .await
            .map_err(|e| anyhow!("数据库错误: {e}"))?
            .ok_or_else(|| anyhow!("User not found"))?;

        Ok((
            UserRole::from_account_type(&user.account_type),
            user.team_id,
        ))
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
        assert!(UserRole::Team.is_team());
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
        assert_eq!(claims.role, UserRole::Admin);
        assert_eq!(claims.team_id, None);
    }
}
