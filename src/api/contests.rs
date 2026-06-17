//! Contest management API.
//!
//! This module provides REST API endpoints for managing contests, including:
//! - Retrieving contest information
//! - Modifying contest start times
//! - Managing scoreboard freeze/thaw times
//!
//! # Endpoints
//!
//! - `GET /api/contests/{id}`: Retrieve contest details
//! - `PATCH /api/contests/{id}`: Modify contest properties (start time or thaw time)
//!
//! # Contest Timing
//!
//! Contests have several important time-related properties:
//! - **start_time**: When the contest begins
//! - **duration**: How long the contest runs
//! - **scoreboard_freeze_duration**: Time before end when scoreboard freezes
//! - **scoreboard_thaw_time**: When to unfreeze the scoreboard after contest end
//!
//! # Safety Restrictions
//!
//! To prevent disruption to live contests, certain operations have restrictions:
//! - Cannot modify start time within 30 seconds of contest start
//! - Cannot set start time to past or within 30 seconds of current time
//! - Thaw time modifications require appropriate contest state

use crate::models::join_tables::contest_group::Entity as ContestGroup;
use crate::models::join_tables::contest_judgement::Entity as ContestJudgement;
use crate::models::join_tables::contest_language::Entity as ContestLanguage;
use crate::models::join_tables::contest_organization::Entity as ContestOrganization;
use crate::models::join_tables::contest_problem::Entity as ContestProblem;
use crate::models::join_tables::contest_run::Entity as ContestRun;
use crate::models::join_tables::contest_submission::Entity as ContestSubmission;
use crate::models::join_tables::contest_team::Entity as ContestTeam;
use crate::models::join_tables::contest_clarification::Entity as ContestClarification;

use crate::models::contests::{Entity as Contest, Model as ContestModel};
use crate::models::groups::{Entity as Group, Model as GroupModel};
use crate::models::judgements::{Entity as JudgementRes, Model as JudgementResModel};
use crate::models::languages::{Entity as Language, Model as LanguageModel};
use crate::models::organizations::{Entity as Organization, Model as OrganizationModel};
use crate::models::problems::{Entity as Problem, Model as ProblemModel};
use crate::models::clarifications::{Entity as Clarification, Model as ClarificationModel};
// use crate::models::team_group::Entity as TeamGroup;
use crate::models::runs::{Entity as RunRes, Model as RunResModel};
use crate::models::submissions::{Entity as Submission, Model as SubmissionModel};
use crate::models::teams::{Entity as Team, Model as TeamModel};
use crate::models::verdicts::{Entity as Judgement, Model as JudgementModel};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use chrono::NaiveDateTime;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};

// #[derive(Debug, Serialize)]
// pub struct ErrorResponse {
//     /// Error message.
//     pub error: String,
// }

/// Retrieves detailed information about a specific contest.
///
/// # Arguments
///
/// * `db` - Database connection extracted from application state
/// * `contest_id` - Path parameter with the contest identifier
///
/// # Returns
///
/// * `Ok(Json<ContestModel>)` - The contest model with all properties
/// * `Err(StatusCode::NOT_FOUND)` - If the contest doesn't exist
/// * `Err(StatusCode::INTERNAL_SERVER_ERROR)` - If database query or validation fails
///
/// # Validation
///
/// The contest model is validated before being returned to ensure data integrity.
///
/// # Examples
///
/// ```bash
/// GET /api/contests/contest123
/// ```
///
/// Response:
/// ```json
/// {
///   "id": "contest123",
///   "name": "ICPC Regional",
///   "formal_name": "ACM ICPC Regional Contest 2026",
///   "start_time": "2026-06-10T09:00:00",
///   "duration": "5:00:00",
///   "scoreboard_type": "icpc"
/// }
/// ```
pub async fn get_contest(
    State(db): State<DatabaseConnection>,
    Path(contest_id): Path<String>,
) -> Result<Json<ContestModel>, StatusCode> {
    // Query the database for the contest
    let contest = Contest::find_by_id(contest_id)
        .one(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    match contest {
        Some(contest) => {
            // Validate the contest model before returning
            contest
                .validate()
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            Ok(Json(contest))
        }
        None => Err(StatusCode::NOT_FOUND),
    }
}

/// Request payload for modifying contest start time.
///
/// This structure supports two related operations:
/// 1. Setting or clearing the contest start time
/// 2. Setting a countdown pause time (only valid when start_time is null)
#[derive(Debug, Deserialize)]
pub struct PatchContestStartRequest {
    /// Contest ID for verification (must match the path parameter)
    pub id: String,
    /// New start time, or null to clear the start time
    pub start_time: Option<NaiveDateTime>,
    /// Countdown pause time in RELTIME format (e.g., "1:23:45")
    ///
    /// Can only be non-null when `start_time` is null. This allows pausing
    /// a countdown timer without committing to a specific start time.
    pub countdown_pause_time: Option<String>,
}

/// Request payload for modifying scoreboard thaw time.
///
/// The scoreboard is typically frozen near the end of a contest to maintain suspense.
/// This endpoint allows setting when the scoreboard should be unfrozen (thawed) to
/// reveal final standings.
#[derive(Debug, Deserialize)]
pub struct PatchContestThawRequest {
    /// Contest ID for verification (must match the path parameter)
    pub id: String,
    /// When to unfreeze the scoreboard
    pub scoreboard_thaw_time: NaiveDateTime,
}

/// Response for thaw time modification with adjusted time.
///
/// Returned with HTTP 200 status when the requested thaw time was in the past
/// and was automatically adjusted to the current time.
// #[derive(Debug, Serialize)]
// pub struct PatchContestThawResponse {
//     /// The contest with updated thaw time
//     pub contest: ContestModel,
// }

/// Modifies contest properties based on the provided payload.
///
/// This endpoint handles multiple types of contest modifications by inspecting the
/// JSON payload and dispatching to the appropriate handler.
///
/// # Supported Operations
///
/// ## Start Time Modification
///
/// Requires `contest_start` capability. Modifies when the contest begins.
///
/// **Payload**: Contains `start_time` and/or `countdown_pause_time`
///
/// **Safety**: Cannot modify if contest has started or will start within 30 seconds
///
/// ## Thaw Time Modification
///
/// Requires `contest_thaw` capability. Sets when the scoreboard unfreezes.
///
/// **Payload**: Contains `scoreboard_thaw_time`
///
/// # Arguments
///
/// * `db` - Database connection extracted from application state
/// * `contest_id` - Path parameter with the contest identifier
/// * `payload` - JSON payload determining the type of modification
///
/// # Returns
///
/// * `Ok((StatusCode::NO_CONTENT, Json(None)))` - Successful modification with no response body
/// * `Ok((StatusCode::OK, Json(Some(contest))))` - Thaw time was adjusted (was in past)
/// * `Err(StatusCode::BAD_REQUEST)` - Invalid payload or ID mismatch
/// * `Err(StatusCode::NOT_FOUND)` - Contest doesn't exist
/// * `Err(StatusCode::FORBIDDEN)` - Operation not allowed in current contest state
///
/// # Examples
///
/// **Modify start time:**
/// ```bash
/// PATCH /api/contests/contest123
/// {
///   "id": "contest123",
///   "start_time": "2026-06-10T09:00:00"
/// }
/// ```
///
/// **Modify thaw time:**
/// ```bash
/// PATCH /api/contests/contest123
/// {
///   "id": "contest123",
///   "scoreboard_thaw_time": "2026-06-10T15:00:00"
/// }
/// ```
pub async fn patch_contest(
    State(db): State<DatabaseConnection>,
    Path(contest_id): Path<String>,
    Json(payload): Json<serde_json::Value>,
) -> Result<(StatusCode, Json<Option<ContestModel>>), StatusCode> {
    // Determine which type of patch this is based on the payload
    if payload.get("start_time").is_some() || payload.get("countdown_pause_time").is_some() {
        // This is a start time modification
        patch_contest_start(db, contest_id, payload).await
    } else if payload.get("scoreboard_thaw_time").is_some() {
        // This is a thaw time modification
        patch_contest_thaw(db, contest_id, payload).await
    } else {
        // Invalid payload
        Err(StatusCode::BAD_REQUEST)
    }
}

/// Handles modification of contest start time.
///
/// This function implements the logic for setting or clearing a contest's start time,
/// with safety checks to prevent modifications to contests that are already running
/// or about to start.
///
/// # Validation Rules
///
/// 1. **ID Verification**: Request ID must match path parameter
/// 2. **Mutual Exclusivity**: Cannot set both `start_time` and `countdown_pause_time`
/// 3. **Contest State**: Cannot modify if contest started or starts within 30 seconds
/// 4. **Time Validity**: New start time cannot be in past or within 30 seconds
///
/// # Arguments
///
/// * `db` - Database connection
/// * `contest_id` - The contest identifier
/// * `payload` - JSON payload with start time modification request
///
/// # Returns
///
/// * `Ok((StatusCode::NO_CONTENT, Json(None)))` - Successful modification
/// * `Err(StatusCode::BAD_REQUEST)` - Invalid request (ID mismatch, validation failure)
/// * `Err(StatusCode::NOT_FOUND)` - Contest doesn't exist
/// * `Err(StatusCode::FORBIDDEN)` - Contest already started or imminent (within 30s)
///
/// # Implementation Status
///
/// **TODO**: Currently validates but doesn't persist changes to database.
/// Needs to update `start_time` and `countdown_pause_time` fields.
async fn patch_contest_start(
    db: DatabaseConnection,
    contest_id: String,
    payload: serde_json::Value,
) -> Result<(StatusCode, Json<Option<ContestModel>>), StatusCode> {
    // TODO: Check that the client has the "contest_start" capability

    // Parse the request payload
    let request: PatchContestStartRequest =
        serde_json::from_value(payload).map_err(|_| StatusCode::BAD_REQUEST)?;

    // Verify the contest ID matches
    if request.id != contest_id {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Validate countdown_pause_time can only be non-null when start_time is null
    if request.start_time.is_some() && request.countdown_pause_time.is_some() {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Fetch the contest from database
    let contest = Contest::find_by_id(&contest_id)
        .one(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let now = chrono::Utc::now().naive_utc();
    let threshold = chrono::Duration::seconds(30);

    // Check if contest is started or within 30s of starting
    if let Some(start_time) = contest.start_time {
        if start_time <= now || (start_time - now) < threshold {
            return Err(StatusCode::FORBIDDEN);
        }
    }

    // If setting a new start time, validate it's not in the past or within 30s
    if let Some(new_start_time) = request.start_time {
        if new_start_time < now || (new_start_time - now) < threshold {
            return Err(StatusCode::FORBIDDEN);
        }
    }

    // TODO: Update the contest in the database with the new start_time and countdown_pause_time
    // For now, return 204 No Content on success
    Ok((StatusCode::NO_CONTENT, Json(None)))
}

/// Handles modification of scoreboard thaw time.
///
/// The scoreboard is typically frozen near the end of a contest to maintain suspense
/// about final rankings. This function sets when the scoreboard should be unfrozen.
///
/// # Behavior
///
/// - If thaw time is in the **past**: Thaws immediately, returns HTTP 200 with updated contest
/// - If thaw time is in the **future**: Schedules thaw, returns HTTP 204 (No Content)
///
/// # Validation Rules
///
/// 1. **ID Verification**: Request ID must match path parameter
/// 2. **Contest State**: Contest must be in a state where thawing is valid
/// 3. **Time Validity**: Thaw time must not be before contest end
/// 4. **Already Thawed**: Cannot thaw an already-thawed contest
///
/// # Arguments
///
/// * `db` - Database connection
/// * `contest_id` - The contest identifier
/// * `payload` - JSON payload with thaw time modification request
///
/// # Returns
///
/// * `Ok((StatusCode::OK, Json(Some(contest))))` - Thawed immediately (time was in past)
/// * `Ok((StatusCode::NO_CONTENT, Json(None)))` - Thaw scheduled for future time
/// * `Err(StatusCode::BAD_REQUEST)` - Invalid request (ID mismatch)
/// * `Err(StatusCode::NOT_FOUND)` - Contest doesn't exist
/// * `Err(StatusCode::FORBIDDEN)` - Operation not allowed (validation failures)
///
/// # Implementation Status
///
/// **TODO**: Currently validates but doesn't persist changes to database.
/// Needs to implement:
/// - Validation of contest state and thaw eligibility
/// - Updating `scoreboard_thaw_time` field in database
/// - Handling immediate vs scheduled thaw
async fn patch_contest_thaw(
    db: DatabaseConnection,
    contest_id: String,
    payload: serde_json::Value,
) -> Result<(StatusCode, Json<Option<ContestModel>>), StatusCode> {
    // TODO: Check that the client has the "contest_thaw" capability

    // Parse the request payload
    let request: PatchContestThawRequest =
        serde_json::from_value(payload).map_err(|_| StatusCode::BAD_REQUEST)?;

    // Verify the contest ID matches
    if request.id != contest_id {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Fetch the contest from database
    let contest = Contest::find_by_id(&contest_id)
        .one(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let now = chrono::Utc::now().naive_utc();
    let thaw_time = request.scoreboard_thaw_time;

    // TODO: Validate that:
    // - The contest can be thawed at the given time
    // - The thaw time is not before the contest end
    // - The contest is not already thawed
    // If any validation fails, return 403 FORBIDDEN

    // If the thaw time is in the past, thaw immediately and return 200 with modified contest
    if thaw_time < now {
        // TODO: Update the contest with current time as thaw time
        // For now, return the contest with 200 status to indicate time was adjusted
        return Ok((StatusCode::OK, Json(Some(contest))));
    }

    // Otherwise, set the thaw time as requested and return 204 No Content
    // TODO: Update the contest in the database with the new scoreboard_thaw_time
    Ok((StatusCode::NO_CONTENT, Json(None)))
}

/// Simplified team response for public access.
///
/// Returns only id and name fields as required by the CCS specification
/// for public/read-only access to contest teams.
#[derive(Debug, Serialize)]
pub struct TeamResponse {
    pub id: String,
    pub name: String,
}

impl From<TeamModel> for TeamResponse {
    fn from(team: TeamModel) -> Self {
        TeamResponse {
            id: team.id,
            name: team.name,
        }
    }
}

/// Retrieves all teams participating in a contest.
///
/// This endpoint returns teams associated with the contest's main scoreboard group.
/// For public access, only basic team information (id, name) is returned.
///
/// # Arguments
///
/// * `db` - Database connection extracted from application state
/// * `contest_id` - Path parameter with the contest identifier
///
/// # Returns
///
/// * `Ok(Json<Vec<TeamResponse>>)` - List of teams in the contest
/// * `Err(StatusCode::NOT_FOUND)` - If the contest doesn't exist
/// * `Err(StatusCode::INTERNAL_SERVER_ERROR)` - If database query fails
///
/// # Examples
///
/// ```bash
/// GET /api/contests/icpc2026/teams
/// ```
///
/// Response:
/// ```json
/// [
///   {"id": "team1", "name": "Team Alpha"},
///   {"id": "team2", "name": "Team Beta"}
/// ]
/// ```
pub async fn get_contest_teams(
    State(db): State<DatabaseConnection>,
    Path(contest_id): Path<String>,
) -> Result<Json<Vec<TeamModel>>, StatusCode> {
    // Verify the contest exists
    Contest::find_by_id(&contest_id)
        .one(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let team_ids: Vec<String> = ContestTeam::find()
        .filter(crate::models::join_tables::contest_team::Column::ContestId.eq(&contest_id))
        .all(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .into_iter()
        .map(|cl| cl.team_id)
        .collect();

    // Fetch the actual problems records
    let teams: Vec<TeamModel> = Team::find()
        .filter(crate::models::teams::Column::Id.is_in(team_ids))
        .all(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(teams))
}

///
/// todo for documents
pub async fn get_contest_team(
    State(db): State<DatabaseConnection>,
    Path((contest_id, team_id)): Path<(String, String)>,
) -> Result<Json<TeamModel>, StatusCode> {
    Contest::find_by_id(&contest_id)
        .one(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let team_ids: Vec<String> = ContestTeam::find()
        .filter(crate::models::join_tables::contest_team::Column::ContestId.eq(&contest_id))
        .all(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .into_iter()
        .map(|cl| cl.team_id)
        .collect();
    if !team_ids.contains(&team_id) {
        return Err(StatusCode::NOT_FOUND);
    }
    let team = Team::find_by_id(team_id)
        .one(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(team))
}

/// Retrieves all judgement types for a contest.
///
/// Returns the complete list of possible judgement responses from the judging system.
/// These represent outcomes like "Accepted", "Wrong Answer", "Time Limit Exceeded", etc.
///
/// # Arguments
///
/// * `db` - Database connection extracted from application state
/// * `contest_id` - Path parameter with the contest identifier
///
/// # Returns
///
/// * `Ok(Json<Vec<JudgementModel>>)` - List of all judgement types
/// * `Err(StatusCode::NOT_FOUND)` - If the contest doesn't exist
/// * `Err(StatusCode::INTERNAL_SERVER_ERROR)` - If database query fails
///
/// # Examples
///
/// ```bash
/// GET /api/contests/icpc2026/judgement-types
/// ```
///
/// Response:
/// ```json
/// [
///   {
///     "id": "AC",
///     "name": "Accepted",
///     "penalty": false,
///     "solved": true,
///     "simplified_judgement_type_id": null
///   },
///   {
///     "id": "WA",
///     "name": "Wrong Answer",
///     "penalty": true,
///     "solved": false,
///     "simplified_judgement_type_id": null
///   }
/// ]
/// ```
pub async fn get_contest_judgement_types(
    State(db): State<DatabaseConnection>,
    Path(contest_id): Path<String>,
) -> Result<Json<Vec<JudgementModel>>, StatusCode> {
    // Verify the contest exists
    let _contest = Contest::find_by_id(&contest_id)
        .one(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Fetch all judgement types
    let judgements = Judgement::find()
        .all(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(judgements))
}

/// Retrieves a specific judgement type for a contest.
///
/// Returns details about a single judgement type identified by its ID
/// (typically a 2-3 letter capitalized shorthand like "AC", "WA", "TLE").
///
/// # Arguments
///
/// * `db` - Database connection extracted from application state
/// * `contest_id` - Path parameter with the contest identifier
/// * `judgement_type_id` - Path parameter with the judgement type identifier
///
/// # Returns
///
/// * `Ok(Json<JudgementModel>)` - The requested judgement type
/// * `Err(StatusCode::NOT_FOUND)` - If the contest or judgement type doesn't exist
/// * `Err(StatusCode::INTERNAL_SERVER_ERROR)` - If database query fails
///
/// # Examples
///
/// ```bash
/// GET /api/contests/icpc2026/judgement-types/AC
/// ```
///
/// Response:
/// ```json
/// {
///   "id": "AC",
///   "name": "Accepted",
///   "penalty": false,
///   "solved": true,
///   "simplified_judgement_type_id": null
/// }
/// ```
pub async fn get_contest_judgement_type(
    State(db): State<DatabaseConnection>,
    Path((contest_id, judgement_type_id)): Path<(String, String)>,
) -> Result<Json<JudgementModel>, StatusCode> {
    // Verify the contest exists
    Contest::find_by_id(&contest_id)
        .one(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Fetch the specific judgement type
    let judgement = Judgement::find_by_id(judgement_type_id)
        .one(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(judgement))
}

/// Retrieves all the languages for a contest.
/// todo for documents
pub async fn get_contest_languages(
    State(db): State<DatabaseConnection>,
    Path(contest_id): Path<String>,
) -> Result<Json<Vec<LanguageModel>>, StatusCode> {
    // Verify the contest exists
    Contest::find_by_id(&contest_id)
        .one(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Find languages associated with this contest through contest_language
    let language_ids: Vec<String> = ContestLanguage::find()
        .filter(crate::models::join_tables::contest_language::Column::ContestId.eq(&contest_id))
        .all(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .into_iter()
        .map(|cl| cl.language_id)
        .collect();

    // Fetch the actual language records
    let languages = Language::find()
        .filter(crate::models::languages::Column::Id.is_in(language_ids))
        .all(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(languages))
}

/// Retrieves a languages for a contest.
/// todo for documents
pub async fn get_contest_language(
    State(db): State<DatabaseConnection>,
    Path((contest_id, language_id)): Path<(String, String)>,
) -> Result<Json<LanguageModel>, StatusCode> {
    // Verify the contest exists
    Contest::find_by_id(&contest_id)
        .one(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let language_ids: Vec<String> = ContestLanguage::find()
        .filter(crate::models::join_tables::contest_language::Column::ContestId.eq(&contest_id))
        .all(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .into_iter()
        .map(|cl| cl.language_id)
        .collect();

    if !language_ids.contains(&language_id) {
        return Err(StatusCode::NOT_FOUND);
    }

    let lang = Language::find_by_id(language_id)
        .one(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(lang))
}

/// Retrieves all problems for a contest.
/// todo for documents
pub async fn get_contest_problems(
    State(db): State<DatabaseConnection>,
    Path(contest_id): Path<String>,
) -> Result<Json<Vec<ProblemModel>>, StatusCode> {
    // Verify the contest exists
    Contest::find_by_id(&contest_id)
        .one(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let problem_ids: Vec<String> = ContestProblem::find()
        .filter(crate::models::join_tables::contest_problem::Column::ContestId.eq(&contest_id))
        .all(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .into_iter()
        .map(|cl| cl.problem_id)
        .collect();

    // Fetch the actual problems records
    let problems: Vec<ProblemModel> = Problem::find()
        .filter(crate::models::problems::Column::Id.is_in(problem_ids))
        .all(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(problems))
}

/// Retrieves a problem for a contest.
/// todo for documents
pub async fn get_contest_problem(
    State(db): State<DatabaseConnection>,
    Path((contest_id, problem_id)): Path<(String, String)>,
) -> Result<Json<ProblemModel>, StatusCode> {
    // Verify the contest exists
    Contest::find_by_id(&contest_id)
        .one(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let problem_ids: Vec<String> = ContestProblem::find()
        .filter(crate::models::join_tables::contest_problem::Column::ContestId.eq(&contest_id))
        .all(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .into_iter()
        .map(|cl| cl.problem_id)
        .collect();

    if !problem_ids.contains(&problem_id) {
        return Err(StatusCode::NOT_FOUND);
    }

    let problem = Problem::find_by_id(problem_id)
        .one(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(problem))
}

/// Retrieves all groups for a contest.
/// todo for documents
pub async fn get_contest_groups(
    State(db): State<DatabaseConnection>,
    Path(contest_id): Path<String>,
) -> Result<Json<Vec<GroupModel>>, StatusCode> {
    // Verify the contest exists
    Contest::find_by_id(&contest_id)
        .one(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let group_ids: Vec<String> = ContestGroup::find()
        .filter(crate::models::join_tables::contest_group::Column::ContestId.eq(&contest_id))
        .all(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .into_iter()
        .map(|cl| cl.group_id)
        .collect();

    // Fetch the actual problems records
    let groups: Vec<GroupModel> = Group::find()
        .filter(crate::models::groups::Column::Id.is_in(group_ids))
        .all(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(groups))
}

///
/// todo for documents
pub async fn get_contest_group(
    State(db): State<DatabaseConnection>,
    Path((contest_id, group_id)): Path<(String, String)>,
) -> Result<Json<GroupModel>, StatusCode> {
    Contest::find_by_id(&contest_id)
        .one(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let group_ids: Vec<String> = ContestGroup::find()
        .filter(crate::models::join_tables::contest_group::Column::ContestId.eq(&contest_id))
        .all(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .into_iter()
        .map(|cl| cl.group_id)
        .collect();
    if !group_ids.contains(&group_id) {
        return Err(StatusCode::NOT_FOUND);
    }
    let group = Group::find_by_id(group_id)
        .one(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(group))
}

/// Retrieves all groups for a contest.
/// todo for documents
pub async fn get_contest_organizations(
    State(db): State<DatabaseConnection>,
    Path(contest_id): Path<String>,
) -> Result<Json<Vec<OrganizationModel>>, StatusCode> {
    // Verify the contest exists
    Contest::find_by_id(&contest_id)
        .one(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let organization_ids: Vec<String> = ContestOrganization::find()
        .filter(crate::models::join_tables::contest_organization::Column::ContestId.eq(&contest_id))
        .all(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .into_iter()
        .map(|cl| cl.organization_id)
        .collect();

    // Fetch the actual problems records
    let organizations: Vec<OrganizationModel> = Organization::find()
        .filter(crate::models::organizations::Column::Id.is_in(organization_ids))
        .all(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(organizations))
}

/// Retrieves all groups for a contest.
/// todo for documents
pub async fn get_contest_organization(
    State(db): State<DatabaseConnection>,
    Path((contest_id, organization_id)): Path<(String, String)>,
) -> Result<Json<OrganizationModel>, StatusCode> {
    // Verify the contest exists
    Contest::find_by_id(&contest_id)
        .one(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let organization_ids: Vec<String> = ContestOrganization::find()
        .filter(crate::models::join_tables::contest_organization::Column::ContestId.eq(&contest_id))
        .all(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .into_iter()
        .map(|cl| cl.organization_id)
        .collect();

    if !organization_ids.contains(&organization_id) {
        return Err(StatusCode::NOT_FOUND);
    }

    // Fetch the actual problems records
    let organization: OrganizationModel = Organization::find_by_id(&organization_id)
        .one(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(organization))
}

/// Retrieves all submissions for a contest.
/// todo for documents
pub async fn get_contest_submissions(
    State(db): State<DatabaseConnection>,
    Path(contest_id): Path<String>,
) -> Result<Json<Vec<SubmissionModel>>, StatusCode> {
    // Verify the contest exists
    Contest::find_by_id(&contest_id)
        .one(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let submission_ids: Vec<String> = ContestSubmission::find()
        .filter(crate::models::join_tables::contest_submission::Column::ContestId.eq(&contest_id))
        .all(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .into_iter()
        .map(|cl| cl.submission_id)
        .collect();

    // Fetch the actual problems records
    let submissions: Vec<SubmissionModel> = Submission::find()
        .filter(crate::models::submissions::Column::Id.is_in(submission_ids))
        .all(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(submissions))
}

/// Retrieves a submissions for a contest.
/// todo for documents
pub async fn get_contest_submission(
    State(db): State<DatabaseConnection>,
    Path((contest_id, submission_id)): Path<(String, i32)>,
) -> Result<Json<SubmissionModel>, StatusCode> {
    // Verify the contest exists
    Contest::find_by_id(&contest_id)
        .one(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let submission_ids: Vec<String> = ContestSubmission::find()
        .filter(crate::models::join_tables::contest_submission::Column::ContestId.eq(&contest_id))
        .all(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .into_iter()
        .map(|cl| cl.submission_id)
        .collect();

    if !submission_ids.contains(&submission_id.to_string()) {
        return Err(StatusCode::NOT_FOUND);
    }

    // Fetch the actual problems records
    let submission: SubmissionModel = Submission::find_by_id(submission_id)
        .one(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(submission))
}

/// Retrieves all submissions for a contest.
/// todo for documents
pub async fn get_contest_judgements(
    State(db): State<DatabaseConnection>,
    Path(contest_id): Path<String>,
) -> Result<Json<Vec<JudgementResModel>>, StatusCode> {
    // Verify the contest exists
    Contest::find_by_id(&contest_id)
        .one(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let judgements_result_ids: Vec<String> = ContestJudgement::find()
        .filter(crate::models::join_tables::contest_judgement::Column::ContestId.eq(&contest_id))
        .all(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .into_iter()
        .map(|cl| cl.judgement_id)
        .collect();

    // Fetch the actual problems records
    let judgements_results: Vec<JudgementResModel> = JudgementRes::find()
        .filter(crate::models::judgements::Column::Id.is_in(judgements_result_ids))
        .all(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(judgements_results))
}

/// Retrieves a submissions for a contest.
/// todo for documents
pub async fn get_contest_judgement(
    State(db): State<DatabaseConnection>,
    Path((contest_id, judgement_id)): Path<(String, i32)>,
) -> Result<Json<JudgementResModel>, StatusCode> {
    // Verify the contest exists
    Contest::find_by_id(&contest_id)
        .one(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let judgements_result_ids: Vec<String> = ContestJudgement::find()
        .filter(crate::models::join_tables::contest_judgement::Column::ContestId.eq(&contest_id))
        .all(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .into_iter()
        .map(|cl| cl.judgement_id)
        .collect();

    if !judgements_result_ids.contains(&judgement_id.to_string()) {
        return Err(StatusCode::NOT_FOUND);
    }

    // Fetch the actual problems records
    let judgement: JudgementResModel = JudgementRes::find_by_id(judgement_id)
        .one(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(judgement))
}

/// todo for documents
///
pub async fn get_contest_runs(
    State(db): State<DatabaseConnection>,
    Path(contest_id): Path<String>,
) -> Result<Json<Vec<RunResModel>>, StatusCode> {
    // Verify the contest exists
    Contest::find_by_id(&contest_id)
        .one(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let run_result_ids: Vec<String> = ContestRun::find()
        .filter(crate::models::join_tables::contest_run::Column::ContestId.eq(&contest_id))
        .all(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .into_iter()
        .map(|cl| cl.run_id)
        .collect();

    // Fetch the actual problems records
    let run_results: Vec<RunResModel> = RunRes::find()
        .filter(crate::models::runs::Column::Id.is_in(run_result_ids))
        .all(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(run_results))
}

/// Retrieves a submissions for a contest.
/// todo for documents
pub async fn get_contest_run(
    State(db): State<DatabaseConnection>,
    Path((contest_id, run_id)): Path<(String, i32)>,
) -> Result<Json<RunResModel>, StatusCode> {
    // Verify the contest exists
    Contest::find_by_id(&contest_id)
        .one(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let run_result_ids: Vec<String> = ContestRun::find()
        .filter(crate::models::join_tables::contest_run::Column::ContestId.eq(&contest_id))
        .all(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .into_iter()
        .map(|cl| cl.run_id)
        .collect();

    if !run_result_ids.contains(&run_id.to_string()) {
        return Err(StatusCode::NOT_FOUND);
    }

    // Fetch the actual problems records
    let runres: RunResModel = RunRes::find_by_id(run_id)
        .one(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(runres))
}

pub async fn get_contest_clarifications(
    State(db): State<DatabaseConnection>,
    Path(contest_id): Path<String>,
) -> Result<Json<Vec<ClarificationModel>>, StatusCode> {
    // Verify the contest exists
    Contest::find_by_id(&contest_id)
        .one(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let clari_ids: Vec<String> = ContestClarification::find()
        .filter(crate::models::join_tables::contest_clarification::Column::ContestId.eq(&contest_id))
        .all(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .into_iter()
        .map(|cl| cl.clarification_id)
        .collect();

    // Fetch the actual problems records
    let clari: Vec<ClarificationModel> = Clarification::find()
        .filter(crate::models::clarifications::Column::Id.is_in(clari_ids))
        .all(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(clari))
}

/// Retrieves a submissions for a contest.
/// todo for documents
pub async fn get_contest_clarification(
    State(db): State<DatabaseConnection>,
    Path((contest_id, clair_id)): Path<(String, i32)>,
) -> Result<Json<ClarificationModel>, StatusCode> {
    // Verify the contest exists
    Contest::find_by_id(&contest_id)
        .one(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let clari_ids: Vec<String> = ContestClarification::find()
        .filter(crate::models::join_tables::contest_clarification::Column::ContestId.eq(&contest_id))
        .all(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .into_iter()
        .map(|cl| cl.clarification_id)
        .collect();

    if !clari_ids.contains(&clair_id.to_string()) {
        return Err(StatusCode::NOT_FOUND);
    }

    // Fetch the actual problems records
    let clari: ClarificationModel = Clarification::find_by_id(clair_id)
        .one(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(clari))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_patch_start_request_validation() {
        // countdown_pause_time can only be non-null when start_time is null
        let json = serde_json::json!({
            "id": "contest1",
            "start_time": "2024-06-25T10:00:00Z",
            "countdown_pause_time": "0:30:00"
        });

        let request: PatchContestStartRequest = serde_json::from_value(json).unwrap();
        assert!(request.start_time.is_some());
        assert!(request.countdown_pause_time.is_some());
        // This should fail validation in the handler
    }

    #[test]
    fn test_patch_thaw_request_parsing() {
        let json = serde_json::json!({
            "id": "contest1",
            "scoreboard_thaw_time": "2024-06-25T15:00:00Z"
        });

        let request: PatchContestThawRequest = serde_json::from_value(json).unwrap();
        assert_eq!(request.id, "contest1");
    }
}
