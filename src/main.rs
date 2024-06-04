use axum::{routing::get, Router};
use axum_prometheus::PrometheusMetricLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

mod routes;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "shortener_rs=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let (prometheus_layer, metrics_handler) = PrometheusMetricLayer::pair();

    let app = Router::new()
        .route("/metrics", get(|| async move { metrics_handler.render() }))
        .route("/health", get(routes::health_check))
        .layer(TraceLayer::new_for_http())
        .layer(prometheus_layer);

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
