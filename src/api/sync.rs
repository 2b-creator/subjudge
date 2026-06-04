use axum::{extract::State, Json};
use sea_orm::{DatabaseConnection, EntityTrait};
use crate::models::{Syncable, team};

pub async fn sync_teams(
    State(db): State<DatabaseConnection>,
    Json(payload): Json<Vec<serde_json::Value>>, // 接收 teams 数组
) -> Result<String, String> {
    println!("接口被触发了！");
    let mut active_models = Vec::new();

    for value in payload {
        // 使用我们之前定义的 Syncable Trait 进行转换
        let active_model = team::Model::from_json(value)
            .map_err(|e| format!("Data format error: {}", e))?;
        active_models.push(active_model);
    }

    // 批量执行插入，并处理已存在的记录（Upsert）
    for model in active_models {
        println!("即将插入的模型: {:?}", model);
        team::Entity::insert(model)
            .on_conflict(
                sea_orm::sea_query::OnConflict::column(team::Column::Id)
                    .update_columns([
                        team::Column::Name,
                        team::Column::Label,
                        team::Column::OrganizationId,
                        team::Column::Resources,
                    ])
                    .to_owned()
            )
            .exec(&db)
            .await
            .map_err(|e| format!("DB Error: {}", e))?;
    }

    Ok("Teams sync completed".to_string())
}