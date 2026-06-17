use sea_orm::entity::prelude::*;


#[derive(Clone, Debug, DeriveEntityModel)]
#[sea_orm(table_name = "team_group")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub team_id: String,
    #[sea_orm(primary_key)]
    pub group_id: String,
}


#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}