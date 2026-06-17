use crate::models::Syncable;
use sea_orm::entity::prelude::*;
use sea_orm::IntoActiveModel;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Clone, Debug, DeriveEntityModel, DeriveActiveModelBehavior, Serialize, Deserialize)]
#[sea_orm(table_name = "teams")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub icpc_id: Option<String>,
    pub name: String,
    pub label: String,
    pub display_name: Option<String>,
    pub organization_id: Option<String>, // 外键关联 Organizations
    
    // 团队状态/位置信息
    pub location: Option<Json>, 
    
    // 动态文件资源列表，采用 JSON 存储
    // 包含 photo, video, backup, key_log, tool_data, desktop, webcam, audio
    pub resources: Json, 
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::organizations::Entity",
        from = "Column::OrganizationId",
        to = "super::organizations::Column::Id"
    )]
    Organization,
}

// ActiveModelBehavior is derived above via DeriveActiveModelBehavior

impl Syncable for Model {
    type ActiveModel = ActiveModel;

    fn from_json(value: serde_json::Value) -> anyhow::Result<Self::ActiveModel> {
        // 1. 反序列化 JSON 到 Model
        // 这里的关键是 Model 中定义了 resources: Json，serde 会自动将 JSON 对象放入该字段
        let mut model: Model = serde_json::from_value(value)?;

        // 2. 检查资源字段完整性 (可选：如果 resources 为空，则填充为空 JSON 对象)
        if model.resources.is_null() {
            model.resources = json!({});
        }

        // 3. 转换为 ActiveModel
        Ok(model.into_active_model())
    }
}