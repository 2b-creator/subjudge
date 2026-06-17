use crate::auth::AuthUser;
use crate::redis_client::RedisClient;
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    Extension,
};
use redis::AsyncCommands;
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
pub async fn get_front(
    auth_user: AuthUser,
    // State(db): State<DatabaseConnection>,
    Extension(redis): Extension<RedisClient>,
) -> Result<Json<Tasks>, (StatusCode, Json<ErrorResponse>)> {
    if !auth_user.role.is_judge() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "Only judgehosts can list accounts".to_string(),
            }),
        ));
    }

    // todo query redis for front of queue
    let mut redis_conn: redis::aio::ConnectionManager = redis.get_connection();
    let task_json: Option<String> = redis_conn
        .lindex("judge_queue", 0)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Redis error: {}", e),
                }),
            )
        })?;

    // 3. 处理没有任务的情况
    let task_str: String = task_json.ok_or((
        StatusCode::NOT_FOUND,
        Json(ErrorResponse {
            error: "No tasks in queue".to_string(),
        }),
    ))?;

    // 4. 将 JSON 字符串反序列化为 Tasks 结构体
    let task: Tasks = serde_json::from_str(&task_str).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to parse task: {}", e),
            }),
        )
    })?;

    Ok(Json(task))
}


pub async fn handle_front(
    auth_user: AuthUser,
    State(db): State<DatabaseConnection>,
    Extension(redis): Extension<RedisClient>,
) -> Result<Json<Tasks>, (StatusCode, Json<ErrorResponse>)> {
    if !auth_user.role.is_judge() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "Only judgehosts can list accounts".to_string(),
            }),
        ));
    }

    // todo query redis for front of queue
    let mut redis_conn: redis::aio::ConnectionManager = redis.get_connection();
    let task_json: Option<String> = redis_conn
        .lpop("judge_queue", None)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Redis error: {}", e),
                }),
            )
        })?;

    // 3. 处理没有任务的情况
    let task_str: String = task_json.ok_or((
        StatusCode::NOT_FOUND,
        Json(ErrorResponse {
            error: "No tasks in queue".to_string(),
        }),
    ))?;

    // 4. 将 JSON 字符串反序列化为 Tasks 结构体
    let task: Tasks = serde_json::from_str(&task_str).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to parse task: {}", e),
            }),
        )
    })?;

    Ok(Json(task))
}
