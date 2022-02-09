use actix_web::{web, HttpResponse};
use sqlx::PgPool;

#[derive(serde::Deserialize)]
pub struct Parameters {
    subscription_token: String,
}

#[allow(clippy::async_yields_async)]
#[tracing::instrument(
    name = "Confirm a pending subscriber",
    skip(parameters)
)]
pub async fn subscribe_confirm(
    parameters: web::Query<Parameters>
) -> HttpResponse {
    log::trace!("{}", parameters.subscription_token);
    HttpResponse::Ok().finish()
}
