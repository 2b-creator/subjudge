mod api;
mod models;

use axum::Router;
use axum::routing::get;
use sea_orm::Database;

#[tokio::main]
async fn main() {
    let db_url = "postgres://subjudge:password@localhost:5432/subjudge_db";
    let db = Database::connect(db_url).await.expect("Failed to connect to DB");

    // 构建 Axum Router 并注入 state
    let api_routes = Router::new()
        .route("/version", get(|| async { "v0.1.0" }))
        .route("/submissions", get(|| async { "List submissions" }))
        .route("/sync/teams", axum::routing::post(api::sync::sync_teams));

    let app = Router::new()
        .nest("/api", api_routes)
        .with_state(db);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
