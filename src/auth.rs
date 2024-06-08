use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::IntoResponse,
};
use axum_prometheus::metrics::counter;
use sha3::{Digest, Sha3_256};
use sqlx::PgPool;

use crate::utils::internal_error;

#[allow(dead_code)]
struct Settings {
    id: String,
    encrypted_api_key: String,
}

pub async fn auth(
    State(pool): State<PgPool>,
    req: Request,
    next: Next,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let labels = [("uri", format!("{}!", req.uri()))];

    let api_key = req
        .headers()
        .get("x-api-key")
        .map(|value| value.to_str().unwrap_or_default())
        .ok_or_else(|| {
            tracing::error!("Unauthorized call: No api key provided");
            counter!("unauthenticated_calls", &labels).increment(1);
            (StatusCode::UNAUTHORIZED, "Unauthorized".into())
        })?;

    let get_api_key_timeout = tokio::time::Duration::from_millis(300);
    let settings = tokio::time::timeout(
        get_api_key_timeout,
        sqlx::query_as!(
            Settings,
            "select id, encrypted_api_key from settings where id = $1",
            "DEFAULT_SETTINGS"
        )
        .fetch_one(&pool),
    )
    .await
    .map_err(internal_error)?
    .map_err(internal_error)?;

    let mut hasher = Sha3_256::new();
    hasher.update(api_key.as_bytes());
    let hashed_api_key = hasher.finalize();

    if settings.encrypted_api_key != format!("{hashed_api_key:x}") {
        tracing::error!("Unauthorized call: Incorrect api key supplied");
        counter!("unauthenticated_calls", &labels).increment(1);
        return Err((StatusCode::UNAUTHORIZED, "Unauthorized".into()));
    }

    Ok(next.run(req).await)
}
