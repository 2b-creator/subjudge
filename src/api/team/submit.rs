use crate::auth::AuthUser;
use crate::models::languages::Entity as Language;
use crate::models::submissions::{ActiveModel as SubmissionActiveModel, Entity as Submission};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use sea_orm::{DatabaseConnection, EntityTrait, ActiveModelTrait, Set};
use serde::{Deserialize, Serialize};
use chrono::Utc;
#[derive(Deserialize)]
pub struct SubmitInfoRequest {
    // pub id: String, // ID
    pub language_id: String,
    pub problem_id: String,
    pub team_id: String,
    pub account_id: Option<String>,
    pub time: String,         // Timestamp of when the submission was made.
    pub contest_time: String, // Real time.
    pub entry_point: Option<String>,
    pub file: serde_json::Value,
    pub reaction: Option<serde_json::Value>, // Reaction video from team's webcam. Only allowed mime types are video/* or application/vnd.apple.mpegurl.
}

/// Error response body.
#[derive(Debug, Serialize)]
pub struct SubmitInfoRespond {
    /// Respond message
    pub message: String,
    pub url: String,
}

/// Error response body.
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    /// Error message.
    pub error: String,
}

/// Submit a solution for a problem
pub async fn submit_solution(
    auth_user: AuthUser,
    State(db): State<DatabaseConnection>,
    Path((contest_id, account_id)): Path<(String, String)>,
    Json(payload): Json<SubmitInfoRequest>,
) -> Result<Json<SubmitInfoRespond>, (StatusCode, Json<ErrorResponse>)> {
    // Check if user is a team member
    if !auth_user.role.is_team() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "You are not allowed to submit.".to_string(),
            }),
        ));
    }

    // Verify that the language exists and is valid
    let _submit_language = Language::find_by_id(&payload.language_id)
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
                    error: format!("Language not found: {}", payload.language_id),
                }),
            )
        })?;

    // Generate unique submission ID using timestamp and nano precision
    let submission_id = format!(
        "{}-{}",
        Utc::now().timestamp(),
        Utc::now().timestamp_subsec_nanos()
    );

    // Create submission active model
    let submission = SubmissionActiveModel {
        id: Set(submission_id.clone()),
        language_id: Set(payload.language_id),
        problem_id: Set(payload.problem_id),
        team_id: Set(payload.team_id),
        account_id: Set(payload.account_id),
        time: Set(payload.time),
        contest_time: Set(payload.contest_time),
        entry_point: Set(payload.entry_point),
        file: Set(payload.file),
        reaction: Set(payload.reaction),
    };

    // Insert submission into database
    submission.insert(&db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to insert submission: {}", e),
            }),
        )
    })?;

    // Generate submission URL
    let submission_url = format!("/api/contests/{}/submissions/{}", contest_id, submission_id);

    Ok(Json(SubmitInfoRespond {
        message: "Submission created successfully!".to_string(),
        url: submission_url,
    }))
}
