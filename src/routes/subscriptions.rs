use actix_web::http::StatusCode;
use actix_web::web::Data;
use actix_web::web::Form;
use actix_web::HttpResponse;
use actix_web::ResponseError;
use anyhow::Context;
use chrono::Utc;
use rand::distributions::Alphanumeric;
use rand::thread_rng;
use rand::Rng;
use serde::Deserialize;
use sqlx::Executor;
use sqlx::PgPool;
use sqlx::Postgres;
use sqlx::Transaction;
use uuid::Uuid;

use crate::domain::NewSubscriber;
use crate::domain::SubscriberEmail;
use crate::domain::SubscriberName;
use crate::email_client::EmailClient;
use crate::startup::ApplicationBaseUrl;

#[derive(Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}

/// Insert a [`NewSubscriber`] into the DB.
///
/// Add the subscriber with a `"pending_confirmation` status, to be confirmed
/// from the [`send_confirmation_email`] function. Registers the query in a
/// transaction for atomicity.
///
/// # Arguments
/// - `new_susbcriber` - The [`NewSubscriber`] to put in the db.
/// - `transaction` - The Postgres transaction to execute on (does not commit).
#[tracing::instrument(
    name = "Saving subscriber into database.",
    skip(new_subscriber, transaction)
)]
async fn insert_subscriber(
    new_subscriber: &NewSubscriber,
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<Uuid, sqlx::Error> {
    let subscriber_id = Uuid::new_v4();
    let query = sqlx::query!(
        r#"
    INSERT INTO subscriptions (id, email, name, subscribed_at, status)
    VALUES ($1, $2, $3, $4, 'pending_confirmation')
    "#,
        subscriber_id,
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        Utc::now()
    );
    transaction.execute(query).await.map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;
    Ok(subscriber_id)
}

impl TryFrom<FormData> for NewSubscriber {
    type Error = String;

    fn try_from(value: FormData) -> Result<Self, Self::Error> {
        let name = SubscriberName::parse(value.name)?;
        let email = SubscriberEmail::parse(value.email)?;
        Ok(Self { email, name })
    }
}

/// Randomly generate a 25 character token for subscriptions.
///
/// For not randomly sending newsletter subscriptions. Up to 10^45 tokens.
fn generate_subscription_token() -> String {
    let mut rng = thread_rng();
    std::iter::repeat_with(|| rng.sample(Alphanumeric))
        .map(char::from)
        .take(25)
        .collect()
}

/// Print out the whole chain of failures that lead to `e`.
///
/// # Arguments
/// * `e` - The error to iterate over.
/// * `f` - The formatter to write with.
pub fn error_chain_fmt(
    e: &impl std::error::Error,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    writeln!(f, "{}\n", e)?;
    let mut current = e.source();
    while let Some(cause) = current {
        writeln!(f, "Caused by:\n\t{}", cause)?;
        current = cause.source();
    }
    Ok(())
}

pub trait IntoHttpError<T> {
    fn http_error(
        self,
        message: &str,
        status_code: StatusCode,
    ) -> core::result::Result<T, actix_web::Error>;

    fn http_internal_error(
        self,
        message: &str,
    ) -> core::result::Result<T, actix_web::Error>
    where
        Self: std::marker::Sized, {
        self.http_error(message, StatusCode::INTERNAL_SERVER_ERROR)
    }
}

/// [`sqlx::Error`] wrapper to prevent implementing foreign traits for foreign
/// types.
pub struct StoreTokenError(sqlx::Error);

impl std::fmt::Display for StoreTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "A database error was encountered while trying to store a \
             subscription token."
        )
    }
}

impl std::error::Error for StoreTokenError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.0)
    }
}

impl std::fmt::Debug for StoreTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

/// Error type for Subscribing.
#[derive(thiserror::Error)]
pub enum SubscribeError {
    #[error("{0}")]
    ValidationError(String),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for SubscribeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for SubscribeError {
    fn status_code(&self) -> StatusCode {
        match self {
            SubscribeError::ValidationError(_) => StatusCode::BAD_REQUEST,
            SubscribeError::UnexpectedError(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }
}

/// Take the given `subscription_token` and `subscriber_id` and place it into
/// the DB for confirmation.
#[tracing::instrument(
    name = "Store subscription token in the database",
    skip(subscription_token, transaction)
)]
pub async fn store_token(
    transaction: &mut Transaction<'_, Postgres>,
    subscriber_id: Uuid,
    subscription_token: &str,
) -> Result<(), StoreTokenError> {
    let query = sqlx::query!(
        r#"
        INSERT INTO subscription_tokens (subscription_token, subscriber_id) 
        VALUES ($1, $2)"#,
        subscription_token,
        subscriber_id
    );
    transaction.execute(query).await.map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        StoreTokenError(e)
    })?;
    Ok(())
}

/// Add a new subscriber to the database and send a confirmation email to them.
///
/// * Take a `Form` and parse it into a [`NewSubscriber`].
/// * Insert the subscriber as `pending_confirmation`.
/// * Generate a subscription token for confirmation and send a confirmation
///   email out.
/// * Commit the transaction to the DB.
/// * Returns a `200` status code.
///
/// # Arguments
/// * `form` - The [`FormData`] to parse into a `NewSubscriber`.
/// * `connection`
#[tracing::instrument(
    name = "Adding a new subscriber", 
    skip(form, connection, email_client, base_url),
    fields(
        subscriber_email = %form.email,
        subscriber_name = %form.name,
    ))]
pub async fn subscribe(
    form: Form<FormData>,
    connection: Data<PgPool>,
    email_client: Data<EmailClient>,
    base_url: Data<ApplicationBaseUrl>,
) -> Result<HttpResponse, SubscribeError> {
    let new_subscriber =
        form.0.try_into().map_err(SubscribeError::ValidationError)?;
    let mut transaction = connection
        .begin()
        .await
        .context("Failed to acquire a Postgres connection from the pool")?;
    let subscriber_id = insert_subscriber(&new_subscriber, &mut transaction)
        .await
        .context("Failed to insert new subscriber in the database.")?;
    let subscription_token = generate_subscription_token();
    store_token(&mut transaction, subscriber_id, &subscription_token)
        .await
        .context("Failed to store confirmation token for new subscriber.")?;
    transaction
        .commit()
        .await
        .context("Failed to commit SQL transaction to store new subscriber.")?;
    send_confirmation_email(
        &email_client,
        new_subscriber,
        &base_url.0,
        &subscription_token,
    )
    .await
    .context("Failed to send a confirmation email.")?;
    Ok(HttpResponse::Ok().finish())
}

/// Sends confirmation email to the `new_subscriber` for the newsletter.
///
/// Given a `subscription_token` for the `new_subscriber`, generate a link for
/// them to click.
///
/// # Arguments
/// * `email_client` - The [`EmailClient`] to send the email from.
/// * `new_subscriber` - The [`NewSubscriber`] to send the email to.
/// * `base_url` - The app's base url for generating the confirmation link.
/// * `subscription_token` - The subscriber's token for confirmation.
#[tracing::instrument(
    name = "Send a confirmation email to a new subscriber.",
    skip(email_client, new_subscriber, base_url)
)]
pub async fn send_confirmation_email(
    email_client: &EmailClient,
    new_subscriber: NewSubscriber,
    base_url: &str,
    subscription_token: &str,
) -> Result<(), reqwest::Error> {
    let confirmation_link = format!(
        "{}/subscriptions/confirm?subscription_token={}",
        base_url, subscription_token
    );
    let html_body = &format!(
        "Welcome to our newsletter!<br />Click <a href=\"{}\">here</a> to \
         confirm your subscription.",
        confirmation_link
    );
    let plain_body = &format!(
        "Welcome to our newsletter!\nVisit {} to confirm your subscription.",
        confirmation_link
    );
    email_client
        .send_email(&new_subscriber.email, "Welcome!", &html_body, &plain_body)
        .await
}
