use crate::models::Syncable;
use sea_orm::IntoActiveModel;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "runs")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i32, // ID
    pub judgement_id: String,
    pub ordinal: i32,
    pub judgement_type_id: String,
    pub time: String,
    pub contest_time: String,
    pub run_time: f32, // Run time in seconds. Should be a non-negative integer multiple of 0.001. The reason for this is to not have rounding ambiguities while still using the natural unit of seconds.
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::judgements::Entity",
        from = "Column::JudgementId",
        to = "super::judgements::Column::Id"
    )]
    Judgements,
    #[sea_orm(
        belongs_to = "super::verdicts::Entity",
        from = "Column::JudgementTypeId",
        to = "super::verdicts::Column::Id"
    )]
    Verdicts,
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
