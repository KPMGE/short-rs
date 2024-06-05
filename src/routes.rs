use axum::{
    body::Body,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response}, Json,
};
use sqlx::PgPool;
use url::Url;

use crate::utils::internal_error;

const DEFAULT_CACHE_CONTROL_HEADER: &str =
    "public, max-age=300, s-maxage=300, stale-while-revalite=300, stale-if-error=300";

pub async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, "Service healthy")
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Link {
    pub id: String,
    pub target_url: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkTarget {
    pub target_url: String
}

pub async fn redirect(
    State(pool): State<PgPool>,
    Path(request_link): Path<String>,
) -> Result<Response, (StatusCode, String)> {
    let select_timeout = tokio::time::Duration::from_millis(300);

    let link = tokio::time::timeout(
        select_timeout,
        sqlx::query_as!(
            Link,
            "select id, target_url from links where id = $1",
            request_link
        )
        .fetch_optional(&pool),
    )
    .await
    .map_err(internal_error)?
    .map_err(internal_error)?
    .ok_or_else(|| "Not found".to_string())
    .map_err(|err| (StatusCode::OK, err))?;

    tracing::debug!("Redirect link id {} to {}", request_link, link.target_url);

    Ok(Response::builder()
        .status(StatusCode::TEMPORARY_REDIRECT)
        .header("Location", link.target_url)
        .header("Cache-Control", DEFAULT_CACHE_CONTROL_HEADER)
        .body(Body::empty())
        .expect("This response should always be constructable"))
}
