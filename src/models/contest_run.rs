use sea_orm::entity::prelude::*;


#[derive(Clone, Debug, DeriveEntityModel)]
#[sea_orm(table_name = "contest_run")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub contest_id: String,
    #[sea_orm(primary_key)]
    pub run_id: String,
}


#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}