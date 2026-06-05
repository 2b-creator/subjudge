mod api;
mod models;

use axum::Router;
use axum::routing::get;
use sea_orm::Database;

#[tokio::main]
async fn main() {
    let db_url = "postgres://subjudge:password@localhost:5432/subjudge_db";
    let db = Database::connect(db_url)
        .await
        .expect("Failed to connect to DB");

    let api_routes = Router::new()
        .route("/version", get(|| async { "v0.1.0" }))
        .route("/submissions", get(|| async { "List submissions" }))
        .route("/contests/{id}", get(api::contests::get_contest))
        .route(
            "/contests/{id}",
            axum::routing::patch(api::contests::patch_contest),
        )
        .route("/contests/{id}/access", get(api::access::get_access))
        .route("/contests/{id}/teams", get(api::contests::get_contest_teams))
        .route("/contests/{id}/judgement-types", get(api::contests::get_contest_judgement_types))
        .route("/contests/{id}/judgement-types/{judgement_type_id}", get(api::contests::get_contest_judgement_type))
        .route("/sync/teams", axum::routing::post(api::sync::sync_teams))
        .route("/sync/groups", axum::routing::post(api::sync::sync_groups))
        .route(
            "/sync/contests",
            axum::routing::post(api::sync::sync_contests),
        )
        .route(
            "/sync/organizations",
            axum::routing::post(api::sync::sync_organizations),
        );

    let app = Router::new().nest("/api", api_routes).with_state(db);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
