use actix_web::web;
use actix_web::HttpResponse;
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::NewSubscriber;
use crate::domain::SubscriberEmail;
use crate::domain::SubscriberName;
use crate::email_client::EmailClient;

#[allow(dead_code)]
#[derive(serde::Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}

impl TryFrom<FormData> for NewSubscriber {
    type Error = String;

    fn try_from(form: FormData) -> Result<Self, Self::Error> {
        let email = SubscriberEmail::parse(form.email)?;
        let name = SubscriberName::parse(form.name)?;
        Ok(NewSubscriber { email, name })
    }
}

#[allow(clippy::async_yields_async)]
#[tracing::instrument(
    name = "Adding a new subscriber ",
    skip(form, connection_pool, email_client),
    fields(
        subscriber_email = %form.email,
        subscriber_name = %form.name
    )
)]
pub async fn subscribe(
    form: web::Form<FormData>,
    connection_pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
) -> HttpResponse {
    let new_subscriber = match form.into_inner().try_into() {
        Ok(new_subscriber) => new_subscriber,
        Err(err) => {
            tracing::error!("{}", err);
            return HttpResponse::BadRequest().body(err);
        }
    };

    if insert_subscriber(&connection_pool, &new_subscriber).await.is_err() {
        return HttpResponse::InternalServerError().finish();
    };

    // send a (uselses) email to the new subscriber
    if email_client.send_email(
        new_subscriber.email,
        "Welcome",
        "Welcome to our newsletter",
        "Welcome to our newsletter",
    ).await.is_err() {
        return HttpResponse::InternalServerError().finish();
    };

    HttpResponse::Ok().finish()
}

#[tracing::instrument(
    name = "Saving new subscriber detail in the database",
    skip(new_subscriber, connection_pool)
)]
pub async fn insert_subscriber(
    connection_pool: &PgPool,
    new_subscriber: &NewSubscriber,
) -> Result<(), sqlx::Error> {
    let _a = sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at, status)
        VALUES ($1, $2, $3, $4, 'confirmed')
        "#,
        Uuid::new_v4(),
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        Utc::now()
    )
    .execute(connection_pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;
    Ok(())
}
