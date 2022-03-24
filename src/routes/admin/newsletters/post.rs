use actix_web::http::header::{self, HeaderValue};
use actix_web::http::StatusCode;
use actix_web::web::ReqData;
use actix_web::{web, HttpResponse, ResponseError};
use actix_web_flash_messages::FlashMessage;
use anyhow::Context;

use sqlx::PgPool;

use crate::authentication::middleware::UserId;

use crate::domain::SubscriberEmail;
use crate::email_client::EmailClient;
use crate::idempotency::{
    get_saved_response, save_response, try_processing, IdempotencyKey,
    NextAction,
};
use crate::routes::error_chain_fmt;

use crate::utils::{e400, e500, see_other};

#[derive(thiserror::Error)]
pub enum PublishError {
    #[error("Authentication failed.")]
    AuthError(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for PublishError {
    fn error_response(&self) -> HttpResponse {
        match self {
            PublishError::UnexpectedError(_) => {
                HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR)
            }
            // Return a 401 for auth errors
            PublishError::AuthError(_) => {
                let mut response = HttpResponse::new(StatusCode::UNAUTHORIZED);
                let header_value =
                    HeaderValue::from_str(r#"Basic realm="publish""#).unwrap();
                response
                    .headers_mut()
                    // actix_web::http::header provides a collection of constants
                    // for the names of several well-known/standard HTTP headers
                    .insert(header::WWW_AUTHENTICATE, header_value);
                response
            }
        }
    }
}

#[tracing::instrument(
    name = "Publish a newsletter issue."
    skip(form, pool, email_client, user_id),
    fields(user_id=%*user_id)
)]
pub async fn publish_newsletter(
    form: web::Form<FormData>,
    user_id: ReqData<UserId>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = *user_id.into_inner();
    // We must destructure the form to avoid upsetting the borrow checker
    let FormData {
        title,
        text_content,
        html_content,
        idempotency_key,
    } = form.into_inner();

    let idempotency_key: IdempotencyKey =
        idempotency_key.try_into().map_err(e400)?;

    let transaction = match try_processing(&pool, &idempotency_key, user_id)
        .await
        .map_err(e500)?
    {
        NextAction::StartProcessing(t) => t,
        NextAction::ReturnSavedResponse(response) => {
            success_message().send();
            return Ok(response);
        }
    };

    let subscribers = get_confirmed_subscribers(&pool).await.map_err(e500)?;
    for subscriber in subscribers {
        match subscriber {
            Ok(subscriber) => {
                email_client
                    .send_email(
                        &subscriber.email,
                        &title,
                        &html_content,
                        &text_content,
                    )
                    .await
                    .with_context(|| {
                        format!(
                            "failed to send newsletter issue to {}",
                            subscriber.email
                        )
                    })
                    .map_err(e500)?;
            }
            Err(err) => {
                tracing::warn!(
                    // We record the error chain as a structured field
                    // on the log record
                    err.cause_chain = ?err, // what is this!?
                    err.message = %err,
                    "Skipping a confirmed subscriber. \
                    Their stored contact is invalid",
                );
            }
        }
    }

    success_message().send();
    // save the saved response
    let response = see_other("/admin/newsletters");
    let response = save_response(transaction, &idempotency_key, user_id, response)
        .await
        .map_err(e500)?;
    Ok(response)
}

fn success_message() -> FlashMessage {
    FlashMessage::info("The newsletter issue has been published!")
}

// fn basic_authentication(
//     headers: &HeaderMap,
// ) -> Result<Credentials, anyhow::Error> {
//     // Header value, if present, must be valid UTF8 string
//     let header_value = headers
//         .get("Authorization")
//         .context("The 'Authorization' header was missing")?
//         .to_str()
//         .context("The 'Authorization' header was not a valid UTF8 string.")?;

//     let base64encoded_segment = header_value
//         .strip_prefix("Basic ")
//         .context("The authorization scheme was not 'Basic'.")?;

//     let decoded_bytes =
//         base64::decode_config(base64encoded_segment, base64::STANDARD)
//             .context("Failed to base64-decode 'Basic' credentials")?;

//     let decoded_credentials = String::from_utf8(decoded_bytes)
//         .context("The decoded credential string is not valid UTF8")?;

//     // split into two segments, using ':' as delimitator
//     let mut credentials = decoded_credentials.splitn(2, ':');

//     let username = credentials
//         .next()
//         .ok_or_else(|| {
//             anyhow::anyhow!("A username must be provided in 'Basic' auth.")
//         })?
//         .to_string();

//     let password = credentials
//         .next()
//         .ok_or_else(|| {
//             anyhow::anyhow!("A password must be provided in 'Basic' auth.")
//         })?
//         .to_string();

//     Ok(Credentials {
//         username,
//         password: Secret::new(password),
//     })
// }

#[derive(serde::Deserialize, serde::Serialize)]
pub struct FormData {
    pub title: String,
    pub html_content: String,
    pub text_content: String,
    // New field
    pub idempotency_key: String,
}

pub struct ConfirmedSubscriber {
    email: SubscriberEmail,
}

#[tracing::instrument(name = "Get confirmed subscriber", skip(pool))]
pub async fn get_confirmed_subscribers(
    pool: &PgPool,
    // We are returning a `Vec` of `Result`s in the happy case.
    // This allows the caller to bubble up errors due to network issues or other
    // transient failures using the `?` operator, while the compiler
    // forces them to handle the subtler mapping error.
    // See http://sled.rs/errors.html for a deep-dive about this technique.
) -> Result<Vec<Result<ConfirmedSubscriber, anyhow::Error>>, anyhow::Error> {
    let confirmed_subscribers = sqlx::query!(
        r#"
            SELECT email FROM subscriptions 
            WHERE status = 'confirmed'
        "#,
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|r| match SubscriberEmail::parse(r.email) {
        Ok(email) => Ok(ConfirmedSubscriber { email }),
        Err(e) => Err(anyhow::anyhow!(e)),
    })
    .collect();
    Ok(confirmed_subscribers)
}
