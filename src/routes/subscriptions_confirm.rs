use actix_web::web;
use actix_web::HttpResponse;
use sqlx::PgPool;
use uuid::Uuid;

/// The request query parameter struct.
#[derive(serde::Deserialize)]
pub struct Parameters {
    subscription_token: String,
}

/// Gets the subscriber ID from the subscription token in the DB.
///
/// # Arguments
/// * `pool` - The Postgres connection pool.
/// * `subscription_token` - The subscription token string to query for.
#[tracing::instrument(
    name = "Get subscriber_id from token",
    skip(subscription_token, pool)
)]
pub async fn get_subscriber_id_from_token(
    pool: &PgPool,
    subscription_token: &str,
) -> Result<Option<Uuid>, sqlx::Error> {
    let result = sqlx::query!(
        "SELECT subscriber_id FROM subscription_tokens WHERE \
         subscription_token = $1",
        subscription_token
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;
    Ok(result.map(|r| r.subscriber_id))
}

/// Updates the DB to confirm the subscriber.
///
/// Given the `subscriber_id`, set the column to `confirmed`.
///
/// # Arguments
/// * `pool` - The Postgres connection pool.
/// * `subscriber_id` - The subscriber to update.
#[tracing::instrument(
    name = "Mark subscriber as confirmed",
    skip(subscriber_id, pool)
)]
pub async fn confirm_subscriber(
    pool: &PgPool,
    subscriber_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"UPDATE subscriptions SET status = 'confirmed' WHERE id = $1"#,
        subscriber_id
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute  query: {:?}", e);
        e
    })?;
    Ok(())
}

/// Route from `"/subscriptions/confirm"` to verify the subscriber.
///
/// Gets the subscriber id from the `parameters`' susbcription token, then
/// confirms it in the DB.
///
/// # Arguments
/// * `parameters` - The request query, deserialized into [`Parameters`].
/// * `pool` - The Postgres connection pool.
#[tracing::instrument(
    name = "Confirm a pending subscriber.",
    skip(parameters, pool)
)]
pub async fn confirm(
    parameters: web::Query<Parameters>,
    pool: web::Data<PgPool>,
) -> HttpResponse {
    let id = match get_subscriber_id_from_token(
        &pool,
        &parameters.subscription_token,
    )
    .await
    {
        Ok(id) => id,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };
    match id {
        None => HttpResponse::Unauthorized().finish(),
        Some(subscriber_id) => {
            if confirm_subscriber(&pool, subscriber_id).await.is_err() {
                return HttpResponse::InternalServerError().finish();
            }
            HttpResponse::Ok().finish()
        }
    }
}
