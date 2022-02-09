use actix_web::{web, HttpResponse};
use sqlx::PgPool;

#[derive(serde::Deserialize)]
pub struct Parameters {
    _subscription_token: String,
}

#[allow(clippy::async_yields_async)]
#[tracing::instrument(
    name = "Confirm a pending subscriber",
    skip(_connection_pool, _parameters)
)]
pub async fn subscribe_confirm(
    _connection_pool: web::Data<PgPool>,
    _parameters: web::Query<Parameters>,
) -> HttpResponse {
    HttpResponse::Ok().finish()
}
