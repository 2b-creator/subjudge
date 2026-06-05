use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create organizations table
        manager
            .create_table(
                Table::create()
                    .table(Organizations::Table)
                    .if_not_exists()
                    .col(string(Organizations::Id).primary_key())
                    .col(string_null(Organizations::IcpcId))
                    .col(string(Organizations::Name))
                    .col(string_null(Organizations::FormalName))
                    .col(string_null(Organizations::Country))
                    .col(string_null(Organizations::CountrySubdivision))
                    .col(string_null(Organizations::Url))
                    .col(string_null(Organizations::TwitterHashtag))
                    .col(string_null(Organizations::TwitterAccount))
                    .col(json_null(Organizations::CountryFlag))
                    .col(json_null(Organizations::CountrySubdivisionFlag))
                    .col(json_null(Organizations::Logo))
                    .col(json_null(Organizations::Location))
                    .to_owned(),
            )
            .await?;

        // Create groups table
        manager
            .create_table(
                Table::create()
                    .table(Groups::Table)
                    .if_not_exists()
                    .col(string(Groups::Id).primary_key())
                    .col(string_null(Groups::IcpcId))
                    .col(string(Groups::Name))
                    .col(string(Groups::Type))
                    .col(string_null(Groups::Location))
                    .to_owned(),
            )
            .await?;

        // Create teams table
        manager
            .create_table(
                Table::create()
                    .table(Teams::Table)
                    .if_not_exists()
                    .col(string(Teams::Id).primary_key())
                    .col(string_null(Teams::IcpcId))
                    .col(string(Teams::Name))
                    .col(string(Teams::Label))
                    .col(string_null(Teams::DisplayName))
                    .col(string_null(Teams::OrganizationId))
                    .col(json_null(Teams::Location))
                    .col(json(Teams::Resources))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_teams_organization")
                            .from(Teams::Table, Teams::OrganizationId)
                            .to(Organizations::Table, Organizations::Id)
                            .on_delete(ForeignKeyAction::SetNull)
                            .on_update(ForeignKeyAction::Cascade)
                    )
                    .to_owned(),
            )
            .await?;

        // Create team_group junction table
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
                            .col(TeamGroup::GroupId)
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_team_group_team")
                            .from(TeamGroup::Table, TeamGroup::TeamId)
                            .to(Teams::Table, Teams::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade)
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_team_group_group")
                            .from(TeamGroup::Table, TeamGroup::GroupId)
                            .to(Groups::Table, Groups::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade)
                    )
                    .to_owned(),
            )
            .await?;

        // Create contests table
        manager
            .create_table(
                Table::create()
                    .table(Contests::Table)
                    .if_not_exists()
                    .col(string(Contests::Id).primary_key())
                    .col(string(Contests::Name))
                    .col(string_null(Contests::FormalName))
                    .col(timestamp_null(Contests::StartTime))
                    .col(string_null(Contests::CountdownPauseTime))
                    .col(string(Contests::Duration))
                    .col(string_null(Contests::ScoreboardFreezeDuration))
                    .col(timestamp_null(Contests::ScoreboardThawTime))
                    .col(string(Contests::ScoreboardType))
                    .col(string_null(Contests::MainScoreboardGroupId))
                    .col(string_null(Contests::PenaltyTime))
                    .col(json_null(Contests::Banner))
                    .col(json_null(Contests::Logo))
                    .col(json_null(Contests::Location))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_contests_main_scoreboard_group")
                            .from(Contests::Table, Contests::MainScoreboardGroupId)
                            .to(Groups::Table, Groups::Id)
                            .on_delete(ForeignKeyAction::SetNull)
                            .on_update(ForeignKeyAction::Cascade)
                    )
                    .to_owned(),
            )
            .await?;

        // Create submissions table
        manager
            .create_table(
                Table::create()
                    .table(Submissions::Table)
                    .if_not_exists()
                    .col(pk_auto(Submissions::Id))
                    .col(string(Submissions::SourceCode))
                    .col(string(Submissions::Status))
                    .col(timestamp_with_time_zone(Submissions::CreatedAt))
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop tables in reverse order to respect foreign key constraints
        manager
            .drop_table(Table::drop().table(Submissions::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(Contests::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(TeamGroup::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(Teams::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(Groups::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(Organizations::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Organizations {
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
enum Groups {
    Table,
    Id,
    IcpcId,
    Name,
    Type,
    Location,
}

#[derive(DeriveIden)]
enum Teams {
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
enum TeamGroup {
    Table,
    TeamId,
    GroupId,
}

#[derive(DeriveIden)]
enum Contests {
    Table,
    Id,
    Name,
    FormalName,
    StartTime,
    CountdownPauseTime,
    Duration,
    ScoreboardFreezeDuration,
    ScoreboardThawTime,
    ScoreboardType,
    MainScoreboardGroupId,
    PenaltyTime,
    Banner,
    Logo,
    Location,
}

#[derive(DeriveIden)]
enum Submissions {
    Table,
    Id,
    SourceCode,
    Status,
    CreatedAt,
}
