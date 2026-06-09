use crate::auth::AuthUser;
use crate::models::languages::Entity as Language;
use crate::models::submissions::ActiveModel as SubmissionActiveModel;
use crate::models::accounts::Entity as Account;
use axum::{
    Json,
    extract::{Multipart, Path, State},
    http::StatusCode,
};
use sea_orm::{DatabaseConnection, EntityTrait, ActiveModelTrait, Set, NotSet};
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
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
#[derive(Debug, Serialize)]
pub struct FileStruct{
    pub href: String,
    pub mime: String,
    pub file: String,
}

/// Error response body.
#[derive(Debug, Serialize)]
pub struct SubmitInfoRespond {
    /// Respond message
    pub language_id: String,
    pub time: String,
    pub contest_time: String,
    pub team_id: String,
    pub problem_id: String,
    pub files: Vec<FileStruct>,
    pub id: String,
    pub entry_point: String,
    pub import_error: String,
    pub message: String,
}

/// Error response body.
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    /// Error message.
    pub error: String,
}

/// Submit a solution for a problem
pub async fn submit_solution_id(
    auth_user: AuthUser,
    State(db): State<DatabaseConnection>,
    Path((contest_id, problem_id)): Path<(String, String)>,
    // Json(payload): Json<SubmitInfoRequest>,
    mut multipart: Multipart,
) -> Result<Json<SubmitInfoRespond>, (StatusCode, Json<ErrorResponse>)> {
    // 1. 定义临时变量接收解析出的数据
    let mut payload: Option<SubmitInfoRequest> = None;
    let mut file_bytes: Option<Vec<u8>> = None;
    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or_default().to_string();
        match name.as_str() {
            "metadata" => {
                let text = field.text().await.map_err(|_| (StatusCode::BAD_REQUEST, Json(ErrorResponse { error: "Invalid metadata".into() })))?;
                payload = Some(serde_json::from_str(&text).map_err(|_| (StatusCode::BAD_REQUEST, Json(ErrorResponse { error: "Invalid JSON".into() })))?);
            }
            "file" => {
                file_bytes = Some(field.bytes().await.map_err(|_| (StatusCode::BAD_REQUEST, Json(ErrorResponse { error: "File upload failed".into() })))?.to_vec());
            }
            _ => {}
        }
    }
    let payload = payload.ok_or((StatusCode::BAD_REQUEST, Json(ErrorResponse { error: "Missing metadata".into() })))?;
    let file_content = file_bytes.ok_or((StatusCode::BAD_REQUEST, Json(ErrorResponse { error: "Missing file".into() })))?;
    // todo for sending judgehost
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

    // Validate contest_time is in ISO 8601 format
    chrono::DateTime::parse_from_rfc3339(&payload.contest_time).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("Invalid contest_time format (must be ISO 8601): {}", e),
            }),
        )
    })?;

    // Validate that team_id is associated with the authenticated account
    let account = Account::find_by_id(&auth_user.user_id)
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
                    error: "Account not found".to_string(),
                }),
            )
        })?;

    // Check if the account's team_id matches the submitted team_id
    if account.team_id.as_ref() != Some(&payload.team_id) {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: format!(
                    "Team ID mismatch: account is associated with team {:?}, but submission is for team {}",
                    account.team_id, payload.team_id
                ),
            }),
        ));
    }

    // Create submission active model
    // Generate file UUID by hashing the base64 encoded file content
    let file_content = if let Some(file_str) = payload.file.as_str() {
        file_str.to_string()
    } else {
        payload.file.to_string()
    };
    let mut hasher = Sha256::new();
    hasher.update(file_content.as_bytes());
    let hash_result = hasher.finalize();
    let file_uuid = hash_result.iter().map(|b| format!("{:02x}", b)).collect::<String>();

    let submission = SubmissionActiveModel {
        id: NotSet,  // Let database auto-generate the i32 ID
        language_id: Set(payload.language_id),
        problem_id: Set(problem_id),
        team_id: Set(payload.team_id),
        account_id: Set(Some(auth_user.user_id.clone())),
        time: Set(payload.time),
        contest_time: Set(payload.contest_time),
        entry_point: Set(payload.entry_point),
        // file: Set(payload.file),
        file_uuid: Set(file_uuid),
        reaction: Set(payload.reaction),
    };

    // Insert submission into database and get the generated ID
    let inserted = submission.insert(&db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to insert submission: {}", e),
            }),
        )
    })?;

    // Construct file struct from the submitted file
    let files = if let Some(file_obj) = inserted.file.as_object() {
        vec![FileStruct {
            href: format!("/api/team/contest/{}/submissions/{}/files", contest_id, inserted.id),
            mime: file_obj.get("mime").and_then(|v| v.as_str()).unwrap_or("application/zip").to_string(),
            file: file_obj.get("data").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        }]
    } else if let Some(file_str) = inserted.file.as_str() {
        vec![FileStruct {
            href: format!("/api/team/contests/{}/submissions/{}/files", contest_id, inserted.id),
            mime: "application/zip".to_string(),
            file: file_str.to_string(),
        }]
    } else {
        vec![]
    };

    Ok(Json(SubmitInfoRespond {
        language_id: inserted.language_id,
        time: inserted.time,
        contest_time: inserted.contest_time,
        team_id: inserted.team_id,
        problem_id: inserted.problem_id,
        files,
        id: inserted.id.to_string(),
        entry_point: inserted.entry_point.unwrap_or_default(),
        import_error: String::new(),
        message: "Submission created successfully!".to_string(),
    }))
}


pub async fn submit_solution(
    auth_user: AuthUser,
    State(db): State<DatabaseConnection>,
    Path(contest_id): Path<String>,
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

    // Validate contest_time is in ISO 8601 format
    chrono::DateTime::parse_from_rfc3339(&payload.contest_time).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("Invalid contest_time format (must be ISO 8601): {}", e),
            }),
        )
    })?;

    // Validate that team_id is associated with the authenticated account
    let account = Account::find_by_id(&auth_user.user_id)
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
                    error: "Account not found".to_string(),
                }),
            )
        })?;

    // Check if the account's team_id matches the submitted team_id
    if account.team_id.as_ref() != Some(&payload.team_id) {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: format!(
                    "Team ID mismatch: account is associated with team {:?}, but submission is for team {}",
                    account.team_id, payload.team_id
                ),
            }),
        ));
    }

    // Create submission active model
    // Generate file UUID by hashing the base64 encoded file content
    let file_content = if let Some(file_str) = payload.file.as_str() {
        file_str.to_string()
    } else {
        payload.file.to_string()
    };
    let mut hasher = Sha256::new();
    hasher.update(file_content.as_bytes());
    let hash_result = hasher.finalize();
    let file_uuid = hash_result.iter().map(|b| format!("{:02x}", b)).collect::<String>();

    let submission = SubmissionActiveModel {
        id: NotSet,  // Let database auto-generate the i32 ID
        language_id: Set(payload.language_id),
        problem_id: Set(payload.problem_id),
        team_id: Set(payload.team_id),
        account_id: Set(Some(auth_user.user_id.clone())),
        time: Set(payload.time),
        contest_time: Set(payload.contest_time),
        entry_point: Set(payload.entry_point),
        file: Set(payload.file),
        file_uuid: Set(file_uuid),
        reaction: Set(payload.reaction),
    };

    // Insert submission into database and get the generated ID
    let inserted = submission.insert(&db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to insert submission: {}", e),
            }),
        )
    })?;

    // Construct file struct from the submitted file
    let files = if let Some(file_obj) = inserted.file.as_object() {
        vec![FileStruct {
            href: format!("/api/team/contests/{}/submissions/{}/files", contest_id, inserted.id),
            mime: file_obj.get("mime").and_then(|v| v.as_str()).unwrap_or("application/zip").to_string(),
            file: file_obj.get("data").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        }]
    } else if let Some(file_str) = inserted.file.as_str() {
        vec![FileStruct {
            href: format!("/api/team/contests/{}/submissions/{}/files", contest_id, inserted.id),
            mime: "application/zip".to_string(),
            file: file_str.to_string(),
        }]
    } else {
        vec![]
    };

    Ok(Json(SubmitInfoRespond {
        language_id: inserted.language_id,
        time: inserted.time,
        contest_time: inserted.contest_time,
        team_id: inserted.team_id,
        problem_id: inserted.problem_id,
        files,
        id: inserted.id.to_string(),
        entry_point: inserted.entry_point.unwrap_or_default(),
        import_error: String::new(),
        message: "Submission created successfully!".to_string(),
    }))
}
