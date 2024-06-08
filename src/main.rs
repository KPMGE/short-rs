mod auth;
mod routes;
mod utils;
mod models;

use axum::middleware;
use axum::routing::{patch, post};
use axum::{routing::get, Router};
use axum_prometheus::PrometheusMetricLayer;
use dotenv::dotenv;
use sqlx::postgres::PgPoolOptions;
use tower_http::trace::TraceLayer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "shortener_rs=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let (prometheus_layer, metrics_handler) = PrometheusMetricLayer::pair();

    let db_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL environment variable must be set!");

    let db_pool = PgPoolOptions::new()
        .max_connections(20)
        .connect(&db_url)
        .await?;

    let app = Router::new()
        // authenticated routes
        .route("/create", post(routes::create_link))
        .route("/:id/statistics", get(routes::get_link_statistics))
        .route("/:id", patch(routes::update_link))
        .route_layer(middleware::from_fn_with_state(db_pool.clone(), auth::auth))
        // unauthenticated routes
        .route("/:id", get(routes::redirect))
        .route("/metrics", get(|| async move { metrics_handler.render() }))
        .route("/health", get(routes::health_check))
        .layer(TraceLayer::new_for_http())
        .layer(prometheus_layer)
        .with_state(db_pool);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3333")
        .await
        .expect("Could not initialize tcp listener!");

    tracing::debug!(
        "Listening on: {}",
        listener
            .local_addr()
            .expect("Could not convert listener address into a local address")
    );

    axum::serve(listener, app)
        .await
        .expect("Could not create http server!");

    Ok(())
}
