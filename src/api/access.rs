//! Contest access control API.
//!
//! This module implements the access control endpoint for contests, providing information
//! about what data and capabilities are available to different types of clients.
//!
//! # Access Levels
//!
//! The API supports three distinct access levels:
//! - **Public**: Unauthenticated users with read-only access to basic contest information
//! - **Team**: Authenticated participants who can submit solutions and view their results
//! - **Admin**: Judges and administrators with full control over the contest
//!
//! # Role-Based Access Control
//!
//! Access is determined by:
//! 1. Client authentication status
//! 2. Client role/permissions
//! 3. Contest state (before/during/after)
//!
//! The response includes:
//! - **capabilities**: Actions the client can perform (e.g., submit, judge, start contest)
//! - **endpoints**: API endpoints available and which properties are visible

use crate::auth::{OptionalAuthUser, UserRole};
use crate::models::access::{AccessResponse, EndpointInfo};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use sea_orm::DatabaseConnection;

/// Retrieves access control information for a specific contest.
///
/// This endpoint returns information about which API endpoints and data properties
/// are visible to the current client, as well as what actions they can perform.
///
/// # Arguments
///
/// * `_db` - Database connection (currently unused, for future authentication)
/// * `contest_id` - Path parameter with the contest identifier
///
/// # Returns
///
/// * `Ok(Json<AccessResponse>)` - Access information including capabilities and available endpoints
/// * `Err(StatusCode)` - Error status if the request fails
///
/// # Response Structure
///
/// ```json
/// {
///   "capabilities": ["team_submit", "clarification_request"],
///   "endpoints": [
///     {
///       "type": "contest",
///       "properties": ["id", "name", "start_time", "duration"]
///     }
///   ]
/// }
/// ```
///
/// # Implementation Status
///
/// ✅ **IMPLEMENTED**: Now uses JWT-based authentication to determine client role.
/// 1. Extracts JWT token from Authorization header
/// 2. Validates token and extracts user role
/// 3. Returns role-specific access information
/// 4. Falls back to public access for unauthenticated requests
///
/// # Authentication
///
/// Include a JWT token in the Authorization header:
/// ```
/// Authorization: Bearer <jwt-token>
/// ```
///
/// # Examples
///
/// ```bash
/// # Unauthenticated (public access)
/// GET /api/contests/contest123/access
///
/// # Authenticated (role-based access)
/// GET /api/contests/contest123/access
/// Authorization: Bearer eyJhbGc...
/// ```
pub async fn get_access(
    State(_db): State<DatabaseConnection>,
    Path(contest_id): Path<String>,
    OptionalAuthUser(auth_user): OptionalAuthUser,
) -> Result<Json<AccessResponse>, StatusCode> {
    // Determine client role from authentication
    let role = match auth_user {
        Some(user) => user.role,
        None => UserRole::Public,
    };

    // Build access response based on authenticated role
    let access = build_access_response(&contest_id, role);

    Ok(Json(access))
}

// Note: ClientRole is now replaced by UserRole from the auth module

/// Builds an AccessResponse based on contest ID and client role.
///
/// This function dispatches to role-specific builders that construct the appropriate
/// access response for each client type.
///
/// # Arguments
///
/// * `contest_id` - The contest identifier (for future role-specific logic)
/// * `role` - The client's role determining their access level
///
/// # Returns
///
/// An `AccessResponse` with capabilities and endpoints appropriate for the role
fn build_access_response(contest_id: &str, role: UserRole) -> AccessResponse {
    match role {
        UserRole::Admin | UserRole::Judge => build_admin_access(contest_id),
        UserRole::Team => build_team_access(contest_id),
        UserRole::Public => build_public_access(contest_id),
    }
}

/// Returns access information for admin/judge clients.
///
/// Admins have full visibility and control over the contest system.
///
/// # Capabilities
///
/// - `contest_start`: Start or modify contest timing
/// - `contest_stop`: Stop or pause the contest
/// - `judge_submission`: Manually judge or rejudge submissions
/// - `clarification_respond`: Respond to team clarification requests
///
/// # Accessible Endpoints
///
/// All contest data endpoints with all properties visible, including:
/// - Full contest configuration
/// - All problems with test data counts
/// - All teams with organization and group memberships
/// - All submissions with detailed timing and source code access
/// - All judgements with complete results
/// - All clarifications (from and to all teams)
fn build_admin_access(_contest_id: &str) -> AccessResponse {
    AccessResponse {
        capabilities: vec![
            "contest_start".to_string(),
            "contest_stop".to_string(),
            "judge_submission".to_string(),
            "clarification_respond".to_string(),
        ],
        endpoints: vec![
            EndpointInfo {
                r#type: "contest".to_string(),
                properties: vec![
                    "id".to_string(),
                    "name".to_string(),
                    "formal_name".to_string(),
                    "start_time".to_string(),
                    "duration".to_string(),
                    "scoreboard_freeze_duration".to_string(),
                    "penalty_time".to_string(),
                ],
            },
            EndpointInfo {
                r#type: "problems".to_string(),
                properties: vec![
                    "id".to_string(),
                    "label".to_string(),
                    "name".to_string(),
                    "ordinal".to_string(),
                    "rgb".to_string(),
                    "color".to_string(),
                    "time_limit".to_string(),
                    "test_data_count".to_string(),
                ],
            },
            EndpointInfo {
                r#type: "teams".to_string(),
                properties: vec![
                    "id".to_string(),
                    "name".to_string(),
                    "organization_id".to_string(),
                    "group_ids".to_string(),
                ],
            },
            EndpointInfo {
                r#type: "organizations".to_string(),
                properties: vec![
                    "id".to_string(),
                    "icpc_id".to_string(),
                    "name".to_string(),
                    "formal_name".to_string(),
                    "country".to_string(),
                    "country_subdivision".to_string(),
                    "url".to_string(),
                    "twitter_hashtag".to_string(),
                    "twitter_account".to_string(),
                    "country_flag".to_string(),
                    "country_subdivision_flag".to_string(),
                    "logo".to_string(),
                    "location".to_string(),
                ],
            },
            EndpointInfo {
                r#type: "groups".to_string(),
                properties: vec!["id".to_string(), "name".to_string(), "type".to_string()],
            },
            EndpointInfo {
                r#type: "submissions".to_string(),
                properties: vec![
                    "id".to_string(),
                    "language_id".to_string(),
                    "problem_id".to_string(),
                    "team_id".to_string(),
                    "time".to_string(),
                    "contest_time".to_string(),
                    "entry_point".to_string(),
                    "reaction".to_string(),
                ],
            },
            EndpointInfo {
                r#type: "judgements".to_string(),
                properties: vec![
                    "id".to_string(),
                    "submission_id".to_string(),
                    "judgement_type_id".to_string(),
                    "start_time".to_string(),
                    "start_contest_time".to_string(),
                    "end_time".to_string(),
                    "end_contest_time".to_string(),
                ],
            },
            EndpointInfo {
                r#type: "clarifications".to_string(),
                properties: vec![
                    "id".to_string(),
                    "from_team_id".to_string(),
                    "to_team_id".to_string(),
                    "reply_to_id".to_string(),
                    "problem_id".to_string(),
                    "text".to_string(),
                    "time".to_string(),
                    "contest_time".to_string(),
                ],
            },
        ],
    }
}

/// Returns access information for team participants.
///
/// Teams have limited access focused on their own contest participation.
///
/// # Capabilities
///
/// - `team_submit`: Submit solutions to problems
/// - `clarification_request`: Request clarifications from judges
///
/// # Accessible Endpoints
///
/// Limited contest data with privacy protections:
/// - Contest information (name, timing, scoring rules)
/// - Problems (without test data counts or internal details)
/// - Teams (only names, not organization/group details)
/// - Own submissions (with basic judgement results)
/// - Own clarifications (cannot see other teams' clarifications)
///
/// # Privacy Restrictions
///
/// Teams cannot see:
/// - Other teams' source code or detailed submission info
/// - Detailed judgement information (test case results)
/// - Organization or group membership details
/// - System timing information
fn build_team_access(_contest_id: &str) -> AccessResponse {
    AccessResponse {
        capabilities: vec![
            "team_submit".to_string(),
            "clarification_request".to_string(),
        ],
        endpoints: vec![
            EndpointInfo {
                r#type: "contest".to_string(),
                properties: vec![
                    "id".to_string(),
                    "name".to_string(),
                    "formal_name".to_string(),
                    "start_time".to_string(),
                    "duration".to_string(),
                    "scoreboard_freeze_duration".to_string(),
                    "penalty_time".to_string(),
                ],
            },
            EndpointInfo {
                r#type: "problems".to_string(),
                properties: vec![
                    "id".to_string(),
                    "label".to_string(),
                    "name".to_string(),
                    "ordinal".to_string(),
                    "rgb".to_string(),
                    "color".to_string(),
                    "time_limit".to_string(),
                ],
            },
            EndpointInfo {
                r#type: "teams".to_string(),
                properties: vec!["id".to_string(), "name".to_string()],
            },
            EndpointInfo {
                r#type: "submissions".to_string(),
                properties: vec![
                    "id".to_string(),
                    "language_id".to_string(),
                    "problem_id".to_string(),
                    "team_id".to_string(),
                    "time".to_string(),
                    "contest_time".to_string(),
                    "reaction".to_string(),
                ],
            },
            EndpointInfo {
                r#type: "judgements".to_string(),
                properties: vec![
                    "id".to_string(),
                    "submission_id".to_string(),
                    "judgement_type_id".to_string(),
                ],
            },
            EndpointInfo {
                r#type: "clarifications".to_string(),
                properties: vec![
                    "id".to_string(),
                    "text".to_string(),
                    "time".to_string(),
                    "problem_id".to_string(),
                ],
            },
        ],
    }
}

/// Returns access information for public/unauthenticated clients.
///
/// Public access provides read-only visibility to basic contest information,
/// typically for spectators viewing public scoreboards or contest listings.
///
/// # Capabilities
///
/// None. Public clients cannot perform any actions.
///
/// # Accessible Endpoints
///
/// Minimal contest data:
/// - Basic contest information (name, timing)
/// - Problem list (labels and names only, no detailed specs)
/// - Team names (no organizational details)
///
/// # Use Cases
///
/// - Public scoreboards
/// - Contest listings
/// - Spectator views
/// - Promotional/informational pages
fn build_public_access(_contest_id: &str) -> AccessResponse {
    AccessResponse {
        capabilities: vec![],
        endpoints: vec![
            EndpointInfo {
                r#type: "contest".to_string(),
                properties: vec![
                    "id".to_string(),
                    "name".to_string(),
                    "formal_name".to_string(),
                    "start_time".to_string(),
                    "duration".to_string(),
                ],
            },
            EndpointInfo {
                r#type: "problems".to_string(),
                properties: vec![
                    "id".to_string(),
                    "label".to_string(),
                    "name".to_string(),
                    "ordinal".to_string(),
                ],
            },
            EndpointInfo {
                r#type: "teams".to_string(),
                properties: vec!["id".to_string(), "name".to_string()],
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_admin_access_has_all_capabilities() {
        let access = build_admin_access("test_contest");
        assert!(access.has_capability("contest_start"));
        assert!(access.has_capability("judge_submission"));
        assert!(access.endpoints.len() > 5);
    }

    #[test]
    fn test_team_access_limited_capabilities() {
        let access = build_team_access("test_contest");
        assert!(access.has_capability("team_submit"));
        assert!(!access.has_capability("contest_start"));
    }

    #[test]
    fn test_public_access_no_capabilities() {
        let access = build_public_access("test_contest");
        assert!(access.capabilities.is_empty());
        assert!(access.has_endpoint("contest"));
        assert!(access.has_endpoint("problems"));
    }

    #[test]
    fn test_referential_integrity() {
        let access = build_team_access("test_contest");

        // If submissions endpoint has team_id property, teams endpoint must exist with id property
        if let Some(submissions) = access.find_endpoint("submissions") {
            if submissions.has_property("team_id") {
                assert!(access.has_endpoint("teams"));
                assert!(access.has_property("teams", "id"));
            }
        }

        // If submissions endpoint has problem_id property, problems endpoint must exist with id property
        if let Some(submissions) = access.find_endpoint("submissions") {
            if submissions.has_property("problem_id") {
                assert!(access.has_endpoint("problems"));
                assert!(access.has_property("problems", "id"));
            }
        }
    }
}
