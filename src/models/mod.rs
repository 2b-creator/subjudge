// src/models/mod.rs
pub mod groups;
pub mod organizations;
// pub mod submission;
pub mod teams;
pub mod team_group;
pub mod access;
pub mod contests;
pub mod judgement;

use sea_orm::ActiveModelTrait;
use serde_json::Value;

pub trait Syncable: Sized {
    type ActiveModel: ActiveModelTrait;
    // 将 JSON 值转换为 SeaORM 的 ActiveModel
    fn from_json(value: Value) -> anyhow::Result<Self::ActiveModel>;
}