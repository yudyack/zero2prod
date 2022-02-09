use actix_web::{web, HttpResponse};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct Parameters {
    subscription_token: String,
}

#[allow(clippy::async_yields_async)]
#[tracing::instrument(
    name = "Confirm a pending subscriber",
    skip(parameters, pool)
)]
pub async fn subscribe_confirm(
    parameters: web::Query<Parameters>,
    pool: web::Data<PgPool>,
) -> HttpResponse {
    // 1. get subscriber_id from subscription_token
    let id = match get_subscriber_id_from_token(
        &pool,
        &parameters.subscription_token,
    )
    .await
    {
        Ok(id) => id,
        Err(err) => {
            tracing::error!("{}", err);
            return HttpResponse::InternalServerError().finish();
        }
    };

    // 2. update subscriber_id to confirmed
    match id {
        None => {
            tracing::error!("No subscriber_id found");
            HttpResponse::Unauthorized().finish()
        }
        Some(id) => {
            if let Err(err) = update_subscriber_to_confirmed(&pool, id).await {
                tracing::error!("{}", err);
                return HttpResponse::InternalServerError().finish();
            }
            HttpResponse::Ok().finish()
        }
    }
}

#[tracing::instrument(
    name = "Get subscriber id from token",
    skip(connection_pool)
)]
pub async fn get_subscriber_id_from_token(
    connection_pool: &PgPool,
    subscription_token: &str,
) -> Result<Option<Uuid>, sqlx::Error> {
    let result = sqlx::query!(
        r#"
        SELECT subscriber_id FROM subscription_token WHERE subscription_token = $1
        "#,
        subscription_token
    )
    .fetch_optional(connection_pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;
    Ok(result.map(|row| row.subscriber_id))
}

#[tracing::instrument(
    name = "Update subscriber to confirmed",
    skip(connection_pool)
)]
pub async fn update_subscriber_to_confirmed(
    connection_pool: &PgPool,
    subscriber_id: Uuid,
) -> Result<(), sqlx::Error> {
    let _result = sqlx::query!(
        r#"
        UPDATE subscriptions SET status = 'confirmed' WHERE id = $1
        "#,
        subscriber_id
    )
    .execute(connection_pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;
    Ok(())
}
