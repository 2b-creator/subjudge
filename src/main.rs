mod api;
mod auth;
mod models;
mod redis_client;

use axum::Router;
use axum::routing::{get, patch, post};
use redis_client::RedisClient;
use sea_orm::Database;

#[tokio::main]
async fn main() {
    let db_url = "postgres://subjudge:password@localhost:5432/subjudge_db";
    let db = Database::connect(db_url)
        .await
        .expect("Failed to connect to DB");

    let redis_url = "redis://127.0.0.1:6379";
    let redis = RedisClient::new(redis_url)
        .await
        .expect("Failed to connect to Redis");

    // Public routes that don't require authentication
    let public_routes = Router::new()
        .route("/version", get(|| async { "v0.1.0" }))
        .route("/auth/health", get(api::auth::health_check))
        // Login endpoint for JWT token generation (alternative auth method)
        .route("/auth/login", axum::routing::post(api::auth::login));

    // Protected routes that require authentication (HTTP Basic Auth or Bearer token)
    let protected_routes = Router::new()
        .route("/auth/me", get(api::auth::get_current_user))
        // Contest endpoints
        .route("/contests/{id}", get(api::contests::get_contest))
        .route(
            "/contests/{id}",
            axum::routing::patch(api::contests::patch_contest),
        )
        .route("/contests/{id}/access", get(api::access::get_access))
        .route(
            "/contests/{id}/teams",
            get(api::contests::get_contest_teams),
        )
        .route(
            "/contests/{id}/teams/{team_id}",
            get(api::contests::get_contest_team),
        )
        .route(
            "/contests/{id}/judgement-types",
            get(api::contests::get_contest_judgement_types),
        )
        .route(
            "/contests/{id}/judgement-types/{judgement_type_id}",
            get(api::contests::get_contest_judgement_type),
        )
        .route(
            "/contests/{id}/languages/",
            get(api::contests::get_contest_languages),
        )
        .route(
            "/contests/{id}/languages/{language_id}",
            get(api::contests::get_contest_language),
        )
        .route(
            "/contests/{id}/problems/",
            get(api::contests::get_contest_problems),
        )
        .route(
            "/contests/{id}/problems/{problem_id}",
            get(api::contests::get_contest_problem),
        )
        .route(
            "/contests/{id}/groups/",
            get(api::contests::get_contest_groups),
        )
        .route(
            "/contests/{id}/groups/{group_id}",
            get(api::contests::get_contest_group),
        )
        .route(
            "/contests/{id}/organizations",
            get(api::contests::get_contest_organizations),
        )
        .route(
            "/contests/{id}/organizations/{organization_id}",
            get(api::contests::get_contest_organization),
        ) // todo for accounts api
        .route(
            "/contests/{id}/submissions",
            get(api::contests::get_contest_submissions),
        )
        .route(
            "/contests/{id}/submissions/{submission_id}",
            get(api::contests::get_contest_submission),
        )
        .route(
            "/contests/{id}/judgements",
            get(api::contests::get_contest_judgements),
        )
        .route(
            "/contests/{id}/judgements/{judgement_id}",
            get(api::contests::get_contest_judgement),
        )
        .route("/contests/{id}/runs", get(api::contests::get_contest_runs))
        .route(
            "/contests/{id}/runs/{run_id}",
            get(api::contests::get_contest_run),
        )
        .route(
            "/contests/{id}/clarifications",
            get(api::contests::get_contest_clarifications),
        )
        .route(
            "/contests/{id}/clarifications/{clarification_id}",
            get(api::contests::get_contest_clarification),
        )
        // team api
        .route(
            "/team/contest/{id}/submissions/",
            get(api::team::submit::submit_solution),
        )
        .route(
            "/team/contest/{id}/submissions/{problem_id}",
            post(api::team::submit::submit_solution_id),
        )
        // Data synchronization endpoints
        .route("/admin/sync/teams", post(api::sync::sync_teams))
        .route("/admin/sync/groups", post(api::sync::sync_groups))
        .route("/admin/sync/contests", post(api::sync::sync_contests))
        .route(
            "/admin/sync/organizations",
            post(api::sync::sync_organizations),
        )
        // Admin endpoints for account management
        .route("/admin/accounts", get(api::admin::accounts::list_accounts))
        .route(
            "/admin/accounts/{account_id}",
            get(api::admin::accounts::get_account_status),
        )
        .route(
            "/admin/accounts/{account_id}/status",
            patch(api::admin::accounts::update_account_status),
        )
        .route(
            "/admin/accounts/{account_id}/disable",
            post(api::admin::accounts::disable_account),
        )
        .route(
            "/admin/accounts/{account_id}/enable",
            post(api::admin::accounts::enable_account),
        )
        // query judge queue
        .route(
            "/judge/tasks/",
            get(api::judge::tasks::get_tasks),
        )
        // .route("/admin/accounts/{account_id}/change-passwd", axum::routing::post(api::admin::accounts::))
        // Apply middleware to inject DB connection for Basic Auth
        .layer(axum::middleware::from_fn_with_state(
            db.clone(),
            auth::inject_db_middleware,
        ));

    let api_routes = Router::new().merge(public_routes).merge(protected_routes);

    let app = Router::new()
        .nest("/api", api_routes)
        .layer(axum::Extension(redis))
        .with_state(db);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
