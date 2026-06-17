use sea_orm::entity::prelude::*;


#[derive(Clone, Debug, DeriveEntityModel)]
#[sea_orm(table_name = "contest_clarification")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub contest_id: String,
    #[sea_orm(primary_key)]
    pub clarification_id: String,
}


#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}