use axum::{routing::get, Router};
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

    let app = Router::new().route("/health", get(routes::health_check));

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
