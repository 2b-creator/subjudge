use crate::auth::AuthUser;
use crate::models::judgements::ActiveModel as JudgementActiveModel;
use crate::models::runs::ActiveModel as RunsActiveModel;
use crate::models::submissions::ActiveModel as SubmissionsActiveModel;
use crate::redis_client::RedisClient;
use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
};
use core::panic::PanicMessage;
use redis::AsyncCommands;
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, NotSet, Set};
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
#[derive(Debug, Serialize, Deserialize)]
pub struct Verdicts {
    pub judgement_id: String,
    pub ordinal: i32, // Ordering of runs in the judgement. Must be different for every run in a judgement. Runs for the same test case must have the same ordinal. Must be between 1 and problem:test_data_count.
    pub judgement_type_id: String,
    pub time: String,
    pub contest_time: String,
    pub run_time: f32,
    pub internal_server_error: bool,
    pub panic_message: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Judgements {
    pub id: i32, // ID
    pub submission_id: i32,
    pub judgement_type_id: Option<String>,
    pub simplified_judgement_type_id: Option<String>,
    pub score: f32,
    pub current: Option<bool>,
    pub start_time: String,         // Absolute time when judgement started.
    pub start_contest_time: String, // Contest relative time when judgement started.
    pub end_time: String,
    pub end_contest_time: String,
    pub max_run_time: Option<f32>,
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
                error: "Only judgehosts can get queue front.".to_string(),
            }),
        ));
    }

    // todo query redis for front of queue
    let mut redis_conn: redis::aio::ConnectionManager = redis.get_connection();
    let task_json: Option<String> = redis_conn.lindex("judge_queue", 0).await.map_err(|e| {
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

pub async fn handle_front_run(
    auth_user: AuthUser,
    State(db): State<DatabaseConnection>,
    Extension(redis): Extension<RedisClient>,
    Json(payload): Json<Vec<Verdicts>>,
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
    let task_json: Option<String> = redis_conn.lpop("judge_queue", None).await.map_err(|e| {
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

    payload.into_iter().for_each(|verdict| {
        let judgements = JudgementActiveModel {
            id: NotSet,
            submission_id: Set(task.submission_id),
            judgement_type_id: Set(Option::from(verdict.judgement_type_id)),
            simplified_judgement_type_id: Set(Option::from(verdict.judgement_type_id)),
        };
    });

    Ok(Json(task))
}
