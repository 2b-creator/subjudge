//! Data synchronization API endpoints.
//!
//! This module provides REST API endpoints for synchronizing external data into the local database.
//! All sync operations use an upsert pattern (insert or update on conflict) to ensure idempotency.
//!
//! # Endpoints
//!
//! - `sync_teams`: Synchronizes team data and their group memberships
//! - `sync_groups`: Synchronizes group data
//! - `sync_contests`: Synchronizes contest data
//! - `sync_organizations`: Synchronizes organization data
//!
//! # Transaction Safety
//!
//! The `sync_teams` function uses database transactions to ensure data integrity when updating
//! teams and their many-to-many relationships with groups. Other sync functions operate on
//! single entities without explicit transactions.

use crate::auth::AuthUser;
use crate::models::{Syncable, groups, teams, contests,organizations};
use crate::models::join_tables::team_group;
use axum::{Json, extract::State};
use sea_orm::{
    ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, TransactionTrait,
};

/// Synchronizes team data and their group memberships from an external source.
///
/// This endpoint accepts a JSON array of team objects and performs the following operations:
///
/// 1. Extracts the `group_ids` field from each team object
/// 2. Upserts the team data (insert or update on conflict)
/// 3. Rebuilds the team-group relationships by deleting existing ones and inserting new ones
///
/// # Arguments
///
/// * `db` - Database connection extracted from application state
/// * `payload` - JSON array of team objects. Each object should contain:
///   - `id`: Team identifier (required)
///   - `name`: Team name
///   - `label`: Team label
///   - `organization_id`: Associated organization ID
///   - `resources`: Team resources (JSON field)
///   - `group_ids`: Array of group IDs this team belongs to (optional)
///
/// # Returns
///
/// * `Ok(String)` - Success message on completion
/// * `Err(String)` - Error message if database operations fail
///
/// # Transaction Safety
///
/// This function uses a database transaction to ensure atomicity. If any operation fails,
/// all changes are rolled back.
///
/// # Examples
///
/// ```json
/// POST /api/sync/teams
/// [
///   {
///     "id": "team1",
///     "name": "Alpha Team",
///     "label": "alpha",
///     "organization_id": "org1",
///     "resources": {},
///     "group_ids": ["group1", "group2"]
///   }
/// ]
/// ```
pub async fn sync_teams(
    auth_user: AuthUser,
    State(db): State<DatabaseConnection>,
    Json(payload): Json<Vec<serde_json::Value>>,
) -> Result<String, String> {
    // Check if user has admin role
    if !auth_user.role.is_admin() {
        return Err("Forbidden: Admin access required".to_string());
    }

    // 1. 使用事务确保数据完整性
    let txn = db.begin().await.map_err(|e| format!("DB Error: {}", e))?;

    for value in payload {
        // A. 提取关联信息，然后从原始 value 中移除该字段，避免传给 Model 报错
        let mut value_map = value.as_object().cloned().ok_or("Invalid JSON object")?;
        let group_ids: Vec<String> = value_map
            .remove("group_ids")
            .and_then(|v: sea_orm::prelude::Json| serde_json::from_value::<Vec<String>>(v).ok())
            .unwrap_or_default();

        // B. 转换剩余数据为 ActiveModel
        let active_model = teams::Model::from_json(serde_json::Value::Object(value_map))
            .map_err(|e: anyhow::Error| format!("Data format error: {}", e))?;

        // C. Upsert Team (注意这里使用 txn 替代 db)
        teams::Entity::insert(active_model)
            .on_conflict(
                sea_orm::sea_query::OnConflict::column(teams::Column::Id)
                    .update_columns([
                        teams::Column::Name,
                        teams::Column::Label,
                        teams::Column::OrganizationId,
                        teams::Column::Resources,
                    ])
                    .to_owned(),
            )
            .exec(&txn)
            .await
            .map_err(|e: sea_orm::prelude::DbErr| format!("DB Error: {}", e))?;

        // D. 同步 TeamGroup 关系 (先删后插)
        let team_id = value
            .get("id")
            .and_then(|v: &sea_orm::prelude::Json| v.as_str())
            .unwrap()
            .to_string();

        team_group::Entity::delete_many()
            .filter(team_group::Column::TeamId.eq(&team_id))
            .exec(&txn)
            .await
            .map_err(|e: sea_orm::prelude::DbErr| format!("DB Error: {}", e))?;

        for g_id in group_ids {
            let relation = team_group::ActiveModel {
                team_id: Set(team_id.clone()),
                group_id: Set(g_id),
            };
            team_group::Entity::insert(relation)
                .exec(&txn)
                .await
                .map_err(|e: sea_orm::prelude::DbErr| format!("DB Error: {}", e))?;
        }
    }

    // 2. 提交事务
    txn.commit().await.map_err(|e: sea_orm::prelude::DbErr| format!("DB Error: {}", e))?;

    Ok("Teams and relations sync completed".to_string())
}

/// Synchronizes group data from an external source.
///
/// This endpoint accepts a JSON array of group objects and upserts them into the database.
/// If a group with the same ID already exists, its `name` and `group_type` fields are updated.
///
/// # Arguments
///
/// * `db` - Database connection extracted from application state
/// * `payload` - JSON array of group objects. Each object should contain:
///   - `id`: Group identifier (required)
///   - `name`: Group name
///   - `group_type`: Type of the group
///
/// # Returns
///
/// * `Ok(String)` - Success message on completion
/// * `Err(String)` - Error message if database operations fail
///
/// # Examples
///
/// ```json
/// POST /api/sync/groups
/// [
///   {
///     "id": "group1",
///     "name": "Admins",
///     "group_type": "system"
///   }
/// ]
/// ```
pub async fn sync_groups(
    auth_user: AuthUser,
    State(db): State<DatabaseConnection>,
    Json(payload): Json<Vec<serde_json::Value>>,
) -> Result<String, String> {
    // Check if user has admin role
    if !auth_user.role.is_admin() {
        return Err("Forbidden: Admin access required".to_string());
    }

    // println!("接口被触发了！");
    let mut active_models = Vec::new();

    for value in payload {
        // 使用我们之前定义的 Syncable Trait 进行转换
        let active_model =
            groups::Model::from_json(value).map_err(|e| format!("Data format error: {}", e))?;
        active_models.push(active_model);
    }

    // 批量执行插入，并处理已存在的记录（Upsert）
    for model in active_models {
        // println!("即将插入的模型: {:?}", model);
        groups::Entity::insert(model)
            .on_conflict(
                sea_orm::sea_query::OnConflict::column(groups::Column::Id)
                    .update_columns([groups::Column::Name, groups::Column::GroupType])
                    .to_owned(),
            )
            .exec(&db)
            .await
            .map_err(|e| format!("DB Error: {}", e))?;
    }

    Ok("Groups sync completed".to_string())
}

/// Synchronizes contest data from an external source.
///
/// This endpoint accepts a JSON array of contest objects and upserts them into the database.
/// If a contest with the same ID already exists, its fields (name, formal_name, start_time,
/// duration, scoreboard_type) are updated.
///
/// # Arguments
///
/// * `db` - Database connection extracted from application state
/// * `payload` - JSON array of contest objects. Each object should contain:
///   - `id`: Contest identifier (required)
///   - `name`: Contest name
///   - `formal_name`: Official contest name
///   - `start_time`: Contest start timestamp
///   - `duration`: Contest duration
///   - `scoreboard_type`: Type of scoreboard to use
///
/// # Returns
///
/// * `Ok(String)` - Success message on completion
/// * `Err(String)` - Error message if database operations fail
///
/// # Examples
///
/// ```json
/// POST /api/sync/contests
/// [
///   {
///     "id": "contest1",
///     "name": "ICPC Regional",
///     "formal_name": "ACM ICPC Regional Contest 2026",
///     "start_time": "2026-06-10T09:00:00Z",
///     "duration": "5:00:00",
///     "scoreboard_type": "icpc"
///   }
/// ]
/// ```
pub async fn sync_contests(
    auth_user: AuthUser,
    State(db): State<DatabaseConnection>,
    Json(payload): Json<Vec<serde_json::Value>>,
) -> Result<String, String> {
    // Check if user has admin role
    if !auth_user.role.is_admin() {
        return Err("Forbidden: Admin access required".to_string());
    }

    // println!("接口被触发了！");
    let mut active_models = Vec::new();

    for value in payload {
        // 使用我们之前定义的 Syncable Trait 进行转换
        let active_model =
            contests::Model::from_json(value).map_err(|e| format!("Data format error: {}", e))?;
        active_models.push(active_model);
    }

    // 批量执行插入，并处理已存在的记录（Upsert）
    for model in active_models {
        // println!("即将插入的模型: {:?}", model);
        contests::Entity::insert(model)
            .on_conflict(
                sea_orm::sea_query::OnConflict::column(contests::Column::Id)
                    .update_columns([
                        contests::Column::Name,
                        contests::Column::FormalName,
                        contests::Column::StartTime,
                        contests::Column::Duration,
                        contests::Column::ScoreboardType,
                    ])
                    .to_owned(),
            )
            .exec(&db)
            .await
            .map_err(|e| format!("DB Error: {}", e))?;
    }

    Ok("Contests sync completed".to_string())
}

/// Synchronizes organization data from an external source.
///
/// This endpoint accepts a JSON array of organization objects and upserts them into the database.
/// If an organization with the same ID already exists, its `name` and `formal_name` fields are updated.
///
/// # Arguments
///
/// * `db` - Database connection extracted from application state
/// * `payload` - JSON array of organization objects. Each object should contain:
///   - `id`: Organization identifier (required)
///   - `name`: Organization name
///   - `formal_name`: Official organization name
///
/// # Returns
///
/// * `Ok(String)` - Success message on completion
/// * `Err(String)` - Error message if database operations fail
///
/// # Examples
///
/// ```json
/// POST /api/sync/organizations
/// [
///   {
///     "id": "org1",
///     "name": "JSUT",
///     "formal_name": "Jiangsu University of Technology"
///   }
/// ]
/// ```
pub async fn sync_organizations(
    auth_user: AuthUser,
    State(db): State<DatabaseConnection>,
    Json(payload): Json<Vec<serde_json::Value>>,
) -> Result<String, String> {
    // Check if user has admin role
    if !auth_user.role.is_admin() {
        return Err("Forbidden: Admin access required".to_string());
    }

    // println!("接口被触发了！");
    let mut active_models = Vec::new();

    for value in payload {
        // 使用我们之前定义的 Syncable Trait 进行转换
        let active_model =
            organizations::Model::from_json(value).map_err(|e| format!("Data format error: {}", e))?;
        active_models.push(active_model);
    }

    // 批量执行插入，并处理已存在的记录（Upsert）
    for model in active_models {
        // println!("即将插入的模型: {:?}", model);
        organizations::Entity::insert(model)
            .on_conflict(
                sea_orm::sea_query::OnConflict::column(organizations::Column::Id)
                    .update_columns([
                        organizations::Column::Name,
                        organizations::Column::FormalName,
                    ])
                    .to_owned(),
            )
            .exec(&db)
            .await
            .map_err(|e| format!("DB Error: {}", e))?;
    }

    Ok("Contests sync completed".to_string())
}
