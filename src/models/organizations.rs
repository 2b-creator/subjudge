use sea_orm::entity::prelude::*;
use sea_orm::IntoActiveModel;
use serde::{Deserialize, Serialize};
use crate::models::Syncable;

#[derive(Clone, Debug, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "organizations")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String, // ID
    pub icpc_id: Option<String>,
    pub name: String,
    pub formal_name: Option<String>,
    pub country: Option<String>, // ISO 3166-1 alpha-3
    pub country_subdivision: Option<String>,
    pub url: Option<String>,
    pub twitter_hashtag: Option<String>,
    pub twitter_account: Option<String>,
    
    pub country_flag: Option<Json>, 
    pub country_subdivision_flag: Option<Json>,
    pub logo: Option<Json>,
    pub location: Option<Json>,
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
        if let Some(ref country) = self.country {
            if country.len() != 3 {
                return Err(anyhow::anyhow!("Country code must be ISO 3166-1 alpha-3"));
            }
        }
        Ok(())
    }
}