use sea_orm::entity::prelude::*;
use sea_orm::IntoActiveModel;
use serde::{Deserialize, Serialize};
use crate::models::Syncable;

#[derive(Clone, Debug, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "contests")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String, // ID
    pub name: String,
    pub formal_name: Option<String>,
    pub start_time: Option<DateTime>, // TIME - scheduled start time
    pub countdown_pause_time: Option<String>, // RELTIME - time left when countdown paused
    pub duration: String, // RELTIME - length of contest (required)
    pub scoreboard_freeze_duration: Option<String>, // RELTIME - how long scoreboard is frozen (default 0:00:00)
    pub scoreboard_thaw_time: Option<DateTime>, // TIME - scheduled thaw time
    pub scoreboard_type: String, // "pass-fail" or "score"
    pub main_scoreboard_group_id: Option<String>, // ID - group representing main scoreboard
    pub penalty_time: Option<String>, // RELTIME - penalty for wrong submission
    pub banner: Option<Json>, // array of FILE - image ~8:1 aspect ratio
    pub logo: Option<Json>, // array of FILE - image ~1:1 aspect ratio
    pub location: Option<Json>, // LOCATION - where contest is held
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

impl Model {
    pub fn validate(&self) -> anyhow::Result<()> {
        // Validate scoreboard_type must be "pass-fail" or "score"
        if self.scoreboard_type != "pass-fail" && self.scoreboard_type != "score" {
            return Err(anyhow::anyhow!(
                "scoreboard_type must be either 'pass-fail' or 'score', got '{}'",
                self.scoreboard_type
            ));
        }
        Ok(())
    }
}