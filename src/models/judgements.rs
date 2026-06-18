use crate::models::Syncable;
use sea_orm::IntoActiveModel;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "judgements")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i32, // ID
    pub submission_id: i32,
    pub judgement_type_id: Option<String>,
    pub simplified_judgement_type_id: Option<String>,
    pub compile_warning: Option<String>,
    pub compile_error: Option<String>,
    pub score: f64,
    pub current: Option<bool>,
    pub start_time: String,         // Absolute time when judgement started.
    pub start_contest_time: String, // Contest relative time when judgement started.
    pub end_time: String,
    pub end_contest_time: String,
    pub max_run_time: Option<f32>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::verdicts::Entity",
        from = "Column::JudgementTypeId",
        to = "super::verdicts::Column::Id"
    )]
    Verdicts,
     #[sea_orm(
        belongs_to = "super::submissions::Entity",
        from = "Column::SubmissionId",
        to = "super::submissions::Column::Id"
    )]
    Submissions,
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
