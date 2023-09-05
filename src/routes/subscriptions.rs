use actix_web::web::Data;
use actix_web::web::Form;
use actix_web::HttpResponse;
use chrono::Utc;
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}

#[tracing::instrument(
    name = "Saving subscriber into database.",
    skip(form, connection)
)]
async fn insert_subscriber(
    form: &FormData,
    connection: &PgPool,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
    INSERT INTO subscriptions (id, email, name, subscribed_at)
    VALUES ($1, $2, $3, $4)
    "#,
        Uuid::new_v4(),
        form.email,
        form.name,
        Utc::now()
    )
    .execute(connection)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;
    Ok(())
}

#[tracing::instrument(
    name = "Adding a new subscriber", 
    skip(form, connection),
    fields(
        subscriber_email = %form.email,
        subscriber_name = %form.name,
    ))]
pub async fn subscribe(
    form: Form<FormData>,
    connection: Data<PgPool>,
) -> HttpResponse {
    match insert_subscriber(&form, &connection).await {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}
