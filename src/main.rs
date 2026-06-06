mod api;
mod auth;
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

    // Public routes that don't require authentication
    let public_routes = Router::new()
        .route("/version", get(|| async { "v0.1.0" }))
        .route("/auth/health", get(api::auth::health_check))
        // Login endpoint for JWT token generation (alternative auth method)
        .route("/auth/login", axum::routing::post(api::auth::login));

    // Protected routes that require authentication (HTTP Basic Auth or Bearer token)
    let protected_routes = Router::new()
        // .route("/submissions", get(|| async { "List submissions" }))
        .route("/auth/me", get(api::auth::get_current_user))
        // Contest endpoints
        .route("/contests/{id}", get(api::contests::get_contest))
        .route(
            "/contests/{id}",
            axum::routing::patch(api::contests::patch_contest),
        )
        .route("/contests/{id}/access", get(api::access::get_access))
        .route("/contests/{id}/teams", get(api::contests::get_contest_teams))
        .route("/contests/{id}/judgement-types", get(api::contests::get_contest_judgement_types))
        .route("/contests/{id}/judgement-types/{judgement_type_id}", get(api::contests::get_contest_judgement_type))
        .route("/contests/{id}/languages/", get(api::contests::get_contest_languages))
        .route("/contests/{id}/languages/{language_id}", get(api::contests::get_contest_language))
        .route("/contests/{id}/problems/", get(api::contests::get_contest_problems))
        .route("/contests/{id}/problems/{problem_id}", get(api::contests::get_contest_problem))
        .route("/contests/{id}/groups/", get(api::contests::get_contest_groups))
        .route("/contests/{id}/groups/{group_id}", get(api::contests::get_contest_group))
        
        
        // Data synchronization endpoints
        .route("/admin/sync/teams", axum::routing::post(api::sync::sync_teams))
        .route("/admin/sync/groups", axum::routing::post(api::sync::sync_groups))
        .route(
            "/admin/sync/contests",
            axum::routing::post(api::sync::sync_contests),
        )
        .route(
            "/admin/sync/organizations",
            axum::routing::post(api::sync::sync_organizations),
        )
        // Admin endpoints for account management
        .route("/admin/accounts", get(api::admin::accounts::list_accounts))
        .route("/admin/accounts/{account_id}", get(api::admin::accounts::get_account_status))
        .route("/admin/accounts/{account_id}/status", axum::routing::patch(api::admin::accounts::update_account_status))
        .route("/admin/accounts/{account_id}/disable", axum::routing::post(api::admin::accounts::disable_account))
        .route("/admin/accounts/{account_id}/enable", axum::routing::post(api::admin::accounts::enable_account))
        // .route("/admin/accounts/{account_id}/change-passwd", axum::routing::post(api::admin::accounts::))        // Apply middleware to inject DB connection for Basic Auth
        .layer(axum::middleware::from_fn_with_state(
            db.clone(),
            auth::inject_db_middleware,
        ));

    let api_routes = Router::new()
        .merge(public_routes)
        .merge(protected_routes);

    let app = Router::new().nest("/api", api_routes).with_state(db);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
