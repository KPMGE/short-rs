use crate::models::*;
use axum::{
    body::Body,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use sqlx::PgPool;
use url::Url;

use crate::utils::internal_error;

const DEFAULT_CACHE_CONTROL_HEADER: &str =
    "public, max-age=300, s-maxage=300, stale-while-revalite=300, stale-if-error=300";

pub async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, "Service healthy")
}

pub async fn create_link(
    State(pool): State<PgPool>,
    Json(new_link): Json<LinkTarget>,
) -> Result<Json<Link>, (StatusCode, String)> {
    let url = Url::parse(&new_link.target_url)
        .map_err(|_| (StatusCode::CONFLICT, "Url malformed".into()))?
        .to_string();

    let new_link_id = crate::utils::generate_id();
    let insert_link_timeout = tokio::time::Duration::from_millis(300);
    let new_link = tokio::time::timeout(
        insert_link_timeout,
        sqlx::query_as!(
            Link,
            r#"
                with inserted_link as (
                    insert into links(id, target_url)
                    values($1, $2)
                    returning id, target_url
                )
                select id, target_url from inserted_link
            "#,
            &new_link_id,
            &url,
        )
        .fetch_one(&pool),
    )
    .await
    .map_err(internal_error)?
    .map_err(internal_error)?;

    tracing::debug!("Created new link with id {} targeting {}", new_link_id, url);

    Ok(Json(new_link))
}

pub async fn update_link(
    State(pool): State<PgPool>,
    Path(link_id): Path<String>,
    Json(new_link): Json<LinkTarget>,
) -> Result<Json<Link>, (StatusCode, String)> {
    let update_timeout = tokio::time::Duration::from_millis(300);

    let url = Url::parse(&new_link.target_url)
        .map_err(|_| (StatusCode::CONFLICT, "Malformed url".to_string()))?
        .to_string();

    let updated_link = tokio::time::timeout(
        update_timeout,
        sqlx::query_as!(
            Link,
            r#"
                with updated_link as (
                    update links set target_url = $1 where id = $2
                    returning id, target_url
                )
                select id, target_url from updated_link
            "#,
            &url,
            &link_id
        )
        .fetch_one(&pool),
    )
    .await
    .map_err(internal_error)?
    .map_err(internal_error)?;

    tracing::debug!(
        "updated link with id {} to target {}",
        &updated_link.id,
        &updated_link.target_url
    );

    Ok(Json(updated_link))
}

pub async fn redirect(
    State(pool): State<PgPool>,
    Path(link_id): Path<String>,
    headers: HeaderMap,
) -> Result<Response, (StatusCode, String)> {
    let select_timeout = tokio::time::Duration::from_millis(300);

    let link = tokio::time::timeout(
        select_timeout,
        sqlx::query_as!(
            Link,
            "select id, target_url from links where id = $1",
            link_id
        )
        .fetch_optional(&pool),
    )
    .await
    .map_err(internal_error)?
    .map_err(internal_error)?
    .ok_or_else(|| "Not found".to_string())
    .map_err(|err| (StatusCode::OK, err))?;

    tracing::debug!("Redirect link id {} to {}", link_id, link.target_url);

    let referer = headers
        .get("referer")
        .map(|value| value.to_str().unwrap_or_default().to_string());

    let user_agent = headers
        .get("user-agent")
        .map(|value| value.to_str().unwrap_or_default().to_string());

    let insert_statistics_timeout = tokio::time::Duration::from_millis(300);
    let saved_statistics = tokio::time::timeout(
        insert_statistics_timeout,
        sqlx::query(
            r#"
                insert into link_statistics (link_is, referer, user_agent)
                values($1, $2, $3)
            "#,
        )
        .bind(&link_id)
        .bind(&referer)
        .bind(&user_agent)
        .execute(&pool),
    )
    .await;

    match saved_statistics {
        Err(elapsed) => tracing::error!(
            "inserting link statistics to database resulted in a timeout {}",
            elapsed
        ),
        Ok(Err(e)) => tracing::error!(
            "inserting link statistics on database resulted in error {}",
            e
        ),
        _ => tracing::debug!("link statatistics inserted correctly"),
    }

    Ok(Response::builder()
        .status(StatusCode::TEMPORARY_REDIRECT)
        .header("Location", link.target_url)
        .header("Cache-Control", DEFAULT_CACHE_CONTROL_HEADER)
        .body(Body::empty())
        .expect("This response should always be constructable"))
}

pub async fn get_link_statistics(
    State(pool): State<PgPool>,
    Path(link_id): Path<String>,
) -> Result<Json<Vec<CountedLinkStatistics>>, (StatusCode, String)> {
    let get_statistics_timeout = tokio::time::Duration::from_millis(300);

    let statistics = tokio::time::timeout(
        get_statistics_timeout,
        sqlx::query_as!(
            CountedLinkStatistics,
            r#"
                select count(*) as amount, user_agent, referer
                from link_statistics 
                group by link_id, user_agent, referer
                having link_id = $1
            "#,
            &link_id
        )
        .fetch_all(&pool),
    )
    .await
    .map_err(internal_error)?
    .map_err(internal_error)?;

    tracing::debug!("link statistics requested for link_id {}", &link_id);

    Ok(Json(statistics))
}
