use actix_web::{http::header::ContentType, web::{self, ReqData}, HttpResponse};
use anyhow::Context;
use reqwest::header::LOCATION;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{session_state::TypedSession, utils::e500, authentication::middleware::UserId};

pub async fn admin_dashboard(
    user_id: ReqData<UserId>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = user_id.into_inner();
    let username = get_username(*user_id, &pool).await.map_err(e500)?;

    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(format!(
            r#"
<html lang="en">
<head>
    <meta http-equiv="content-type" content="text/html; charset=utf-8">
    <title>Admin dashboard</title>
</head>
<body>
    <p>Welcome {}!</p>
    <p>Available actions:</p>
    <ol>
        <li><a href="/admin/password">Change Password</a></li>
        <li><a href="/admin/newsletters">Post Newsletters</a></li>
        <li>
            <a href="javascript:document.logoutForm.submit()">Logout</a>
            <form name="logoutForm" action="/admin/logout" method="POST" hidden>
                <input hidden type="submit" value="Logout">
            </form
    </ol>
</body>
</html>"#,
            username
        )))
}

#[tracing::instrument(skip(pool))]
pub async fn get_username(
    user_id: Uuid,
    pool: &PgPool,
) -> Result<String, anyhow::Error> {
    let row = sqlx::query!(
        r#"
        SELECT username
        FROM users
        WHERE user_id = $1
        "#,
        user_id
    )
    .fetch_one(pool)
    .await
    .context("Failed to performed a query to retrieve a username")?;
    Ok(row.username)
}
