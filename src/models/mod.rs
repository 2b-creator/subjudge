// src/models/mod.rs
pub mod groups;
pub mod organizations;
pub mod teams;
pub mod team_group;
pub mod access;
pub mod contests;
pub mod judgements;
pub mod languages;
pub mod problems;
pub mod accounts;
pub mod submissions;
pub mod verdicts;
pub mod contest_judgement;
pub mod contest_language;
pub mod contest_problem;
pub mod contest_group;
pub mod contest_organization;
pub mod contest_team;
pub mod contest_submission;

use sea_orm::ActiveModelTrait;
use serde_json::Value;

pub trait Syncable: Sized {
    type ActiveModel: ActiveModelTrait;
    // 将 JSON 值转换为 SeaORM 的 ActiveModel
    fn from_json(value: Value) -> anyhow::Result<Self::ActiveModel>;
}