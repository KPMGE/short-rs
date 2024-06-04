use axum::{routing::get, Router};

mod routes;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = Router::new().route("/health", get(routes::health_check));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3333")
        .await
        .expect("Could not initialize tcp listener!");

    axum::serve(listener, app)
        .await
        .expect("Could not create http server!");

    Ok(())
}
