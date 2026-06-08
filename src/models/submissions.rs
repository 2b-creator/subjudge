use sea_orm::entity::prelude::*;
use sea_orm::IntoActiveModel;
use serde::{Deserialize, Serialize};
use crate::models::Syncable;

#[derive(Clone, Debug, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "submissions")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i32, // ID
    pub language_id: String,
    pub problem_id: String,
    pub team_id: String,
    pub account_id: Option<String>,
    pub time: String, // Timestamp of when the submission was made.
    pub contest_time: String, // Real time.
    pub entry_point: Option<String>,
    pub file: Json,
    pub reaction: Option<Json>, // Reaction video from team's webcam. Only allowed mime types are video/* or application/vnd.apple.mpegurl.
}


#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::languages::Entity",
        from = "Column::LanguageId",
        to = "super::languages::Column::Id"
    )]
    Language,

    #[sea_orm(
        belongs_to = "super::problems::Entity",
        from = "Column::ProblemId",
        to = "super::problems::Column::Id"
    )]
    Problem,

    #[sea_orm(
        belongs_to = "super::teams::Entity",
        from = "Column::TeamId",
        to = "super::teams::Column::Id"
    )]
    Team,

    #[sea_orm(
        belongs_to = "super::accounts::Entity",
        from = "Column::AccountId",
        to = "super::accounts::Column::Id"
    )]
    Account,
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