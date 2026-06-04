use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 1. 创建 organizations 表
        manager
            .create_table(
                Table::create()
                    .table(Organization::Table)
                    .if_not_exists()
                    .col(string(Organization::Id).primary_key())
                    .col(string_null(Organization::IcpcId))
                    .col(string(Organization::Name))
                    .col(string_null(Organization::FormalName))
                    .col(string_null(Organization::Country))
                    .col(string_null(Organization::CountrySubdivision))
                    .col(string_null(Organization::Url))
                    .col(string_null(Organization::TwitterHashtag))
                    .col(string_null(Organization::TwitterAccount))
                    .col(json_null(Organization::CountryFlag))
                    .col(json_null(Organization::CountrySubdivisionFlag))
                    .col(json_null(Organization::Logo))
                    .col(json_null(Organization::Location))
                    .to_owned(),
            )
            .await?;

        // 2. 创建 teams 表
        manager
            .create_table(
                Table::create()
                    .table(Team::Table)
                    .if_not_exists()
                    .col(string(Team::Id).primary_key())
                    .col(string_null(Team::IcpcId))
                    .col(string(Team::Name))
                    .col(string(Team::Label))
                    .col(string_null(Team::DisplayName))
                    .col(string_null(Team::OrganizationId))
                    .col(json_null(Team::Location))
                    .col(json(Team::Resources))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_team_organization")
                            .from(Team::Table, Team::OrganizationId)
                            .to(Organization::Table, Organization::Id)
                            .on_delete(ForeignKeyAction::SetNull)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // 3. 创建 groups 表
        manager
            .create_table(
                Table::create()
                    .table(Group::Table)
                    .if_not_exists()
                    .col(string(Group::Id).primary_key())
                    .col(string_null(Group::IcpcId))
                    .col(string(Group::Name))
                    .col(string(Group::Type))
                    .col(string_null(Group::Location))
                    .to_owned(),
            )
            .await?;

        // 4. 创建 team_groups 关联表
        manager
            .create_table(
                Table::create()
                    .table(TeamGroup::Table)
                    .if_not_exists()
                    .col(string(TeamGroup::TeamId))
                    .col(string(TeamGroup::GroupId))
                    .primary_key(
                        Index::create()
                            .col(TeamGroup::TeamId)
                            .col(TeamGroup::GroupId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_team_group_team")
                            .from(TeamGroup::Table, TeamGroup::TeamId)
                            .to(Team::Table, Team::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_team_group_group")
                            .from(TeamGroup::Table, TeamGroup::GroupId)
                            .to(Group::Table, Group::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // 5. 创建 submissions 表
        manager
            .create_table(
                Table::create()
                    .table(Submission::Table)
                    .if_not_exists()
                    .col(pk_auto(Submission::Id))
                    .col(text(Submission::SourceCode))
                    .col(string(Submission::Status))
                    .col(timestamp_with_time_zone(Submission::CreatedAt))
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 按相反顺序删除表
        manager
            .drop_table(Table::drop().table(Submission::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(TeamGroup::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(Group::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(Team::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(Organization::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Organization {
    Table,
    Id,
    IcpcId,
    Name,
    FormalName,
    Country,
    CountrySubdivision,
    Url,
    TwitterHashtag,
    TwitterAccount,
    CountryFlag,
    CountrySubdivisionFlag,
    Logo,
    Location,
}

#[derive(DeriveIden)]
enum Team {
    Table,
    Id,
    IcpcId,
    Name,
    Label,
    DisplayName,
    OrganizationId,
    Location,
    Resources,
}

#[derive(DeriveIden)]
enum Group {
    Table,
    Id,
    IcpcId,
    Name,
    Type,
    Location,
}

#[derive(DeriveIden)]
enum TeamGroup {
    Table,
    TeamId,
    GroupId,
}

#[derive(DeriveIden)]
enum Submission {
    Table,
    Id,
    SourceCode,
    Status,
    CreatedAt,
}
