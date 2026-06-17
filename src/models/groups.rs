use sea_orm::{ActiveValue, entity::prelude::*};
use serde::{Deserialize, Serialize};
use crate::models::Syncable;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "groups")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub icpc_id: Option<String>,
    pub name: String,
    // 使用 sea_orm 的 column_name 属性将数据库字段名映射为 group_type
    #[sea_orm(column_name = "type")] 
    #[serde(rename = "type")]
    pub group_type: String, 
    pub location: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl Syncable for Model {
    type ActiveModel = ActiveModel;

    fn from_json(value: serde_json::Value) -> anyhow::Result<Self::ActiveModel> {
        // 1. 将 JSON 转为 Model
        let model: Model = serde_json::from_value(value)?;
        
        // 2. 转为 ActiveModel 并设置字段为 "已设置" (Set)
        Ok(ActiveModel {
            id: ActiveValue::Set(model.id),
            icpc_id: ActiveValue::Set(model.icpc_id),
            name: ActiveValue::Set(model.name),
            group_type: ActiveValue::Set(model.group_type),
            location: ActiveValue::Set(model.location),
        })
    }
}