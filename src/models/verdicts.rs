use sea_orm::entity::prelude::*;
use sea_orm::IntoActiveModel;
use serde::{Deserialize, Serialize};
use crate::models::Syncable;


#[derive(Clone, Debug, Serialize, Deserialize, DeriveEntityModel)]
#[sea_orm(table_name = "verdicts")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String, // AC, RE, TLE, MLE, WA
    pub name: String, // Wrong Answer, Correct
    pub penalty: i32,
    pub solved: bool,
    pub weight: i32, // Lazy Judge
    pub return_val: i32,
    pub simplified_judgement_type_id: Option<String>,
}


#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    
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