use crate::auth::AuthUser;
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set};
use serde::{Deserialize, Serialize};
/// Error response body.
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    /// Error message.
    pub error: String,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Tasks {
    pub submission_id: i32,
    pub language_id: String,
    pub problem_id: String,
    pub team_id: String,
    pub contest_time: String,
}
pub async fn get_tasks(
    auth_user: AuthUser,
    State(db): State<DatabaseConnection>,
) -> Result<Json<Tasks>, (StatusCode, Json<ErrorResponse>)> {
    // todo query redis.
}
