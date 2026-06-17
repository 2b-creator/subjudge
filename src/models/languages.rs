use sea_orm::entity::prelude::*;
use sea_orm::IntoActiveModel;
use serde::{Deserialize, Serialize};
use crate::models::Syncable;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Command {
    // 基础命令
    pub command: String,
    
    // 参数列表，默认空数组
    #[serde(default)]
    pub args: Vec<String>,
    
    // 版本检查相关
    #[serde(default)]
    pub version: String,
    
    // 获取版本号的命令，默认 "<command> --version"
    pub version_command: Option<String>,
}

impl Command {
    pub fn build_command(&self, files: Vec<String>) -> String {
        let file_str = files.join(" ");
        let cmd = self.command.clone();
        
        // 替换占位符
        let args = self.args.iter()
            .map(|arg| arg.replace("{files}", &file_str))
            .collect::<Vec<String>>()
            .join(" ");
            
        format!("{} {}", cmd, args)
    }
}

#[derive(Clone, Debug, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "languages")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String, // 对应 ID
    pub name: String,
    pub entry_point_required: bool,
    pub entry_point_name: Option<String>,
    pub extensions: Json, // 存储为数组，对应 JSON 的 string array
    pub compiler_command: Json, // 存储为 JSON 对象
    pub runner_command: Json,   // 存储为 JSON 对象
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
