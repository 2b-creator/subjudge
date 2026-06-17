use crate::models::Syncable;
use sea_orm::IntoActiveModel;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

// Default value functions
fn default_string() -> String {
    String::new()
}

fn default_ordinal() -> i32 {
    0
}

fn default_time_limit() -> f32 {
    1.0 // 1 second
}

fn default_memory_limit() -> i32 {
    256 // 256 MB
}

fn default_output_limit() -> i32 {
    64 // 64 MB
}

fn default_code_limit() -> i32 {
    64 // 64 KB
}

fn default_test_data_count() -> i32 {
    0
}

fn default_max_score() -> f32 {
    100.0
}

#[derive(Clone, Debug, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "problems")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String, // ID
    pub uuid: Option<String>,
    #[serde(default = "default_string")]
    pub label: String,
    #[serde(default = "default_string")]
    pub name: String,
    #[serde(default = "default_ordinal")]
    pub ordinal: i32,
    pub rgb: Option<String>,
    pub color: Option<String>,
    #[serde(default = "default_time_limit")]
    pub time_limit: f32,
    #[serde(default = "default_memory_limit")]
    pub memory_limit: i32,
    #[serde(default = "default_output_limit")]
    pub output_limit: i32,
    #[serde(default = "default_code_limit")]
    pub code_limit: i32,
    #[serde(default = "default_test_data_count")]
    pub test_data_count: i32,
    #[serde(default = "default_max_score")]
    pub max_score: f32,
    pub package: Option<Json>,
    pub statement: Option<Json>,
    pub attachments: Option<Json>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

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
