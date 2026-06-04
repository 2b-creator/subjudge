// src/models/mod.rs
pub mod group;
pub mod organization;
// pub mod submission;
pub mod team;
pub mod team_group;

use sea_orm::ActiveModelTrait;
use serde_json::Value;

pub trait Syncable: Sized {
    type ActiveModel: ActiveModelTrait;
    // 将 JSON 值转换为 SeaORM 的 ActiveModel
    fn from_json(value: Value) -> anyhow::Result<Self::ActiveModel>;
}