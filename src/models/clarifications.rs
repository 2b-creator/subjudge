use crate::models::Syncable;
use sea_orm::IntoActiveModel;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "clarifications")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i32, // ID
    pub from_team_id: Option<String>,
    pub to_team_ids: Option<Vec<String>>,
    pub to_group_ids: Option<Vec<String>>,
    pub reply_to_id: Option<i32>,
    pub problem_id: Option<String>,
    pub text: String, // markdown format
    pub time: String,
    pub contest_time: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::problems::Entity",
        from = "Column::ProblemId",
        to = "super::problems::Column::Id"
    )]
    Problem,
    #[sea_orm(
        belongs_to = "super::teams::Entity",
        from = "Column::FromTeamId",
        to = "super::teams::Column::Id"
    )]
    Team,
}

impl ActiveModelBehavior for ActiveModel {}
impl Syncable for Model {
    type ActiveModel = ActiveModel;

    fn from_json(value: serde_json::Value) -> anyhow::Result<Self::ActiveModel> {
        // 1. 利用 serde_json 自动将 Value 反序列化为你的 Model
        let model: Model = serde_json::from_value(value)?;

        // 2. 利用 sea-orm 的 IntoActiveModel 特性自动转换
        Ok(model.into_active_model())
    }
}
