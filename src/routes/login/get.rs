use actix_web::{http::header::ContentType, web, HttpResponse};
use hmac::{Hmac, Mac};
use secrecy::ExposeSecret;

use crate::startup::HmacSecret;

#[derive(serde::Deserialize)]
pub struct QueryParams {
    error: Option<String>,
    tag: Option<String>,
}

impl QueryParams {
    fn verify(self, secret: &HmacSecret) -> Result<String, anyhow::Error> {
        if let QueryParams {
            error: Some(error),
            tag: Some(tag),
        } = self
        {
            let tag = hex::decode(&tag)?;
            let query_string =
                format!("error={}", urlencoding::Encoded::new(&error));

            let mut mac = Hmac::<sha2::Sha256>::new_from_slice(
                secret.0.expose_secret().as_bytes(),
            )
            .unwrap();
            mac.update(query_string.as_bytes());
            mac.verify_slice(&tag)?;

            Ok(error)
        } else {
            Err(anyhow::anyhow!("invalid query params"))
        }
    }
}

pub async fn login_form(
    query: web::Query<QueryParams>,
    secret: web::Data<HmacSecret>,
) -> HttpResponse {
    let error_html = match query.0.verify(&secret) {
        Ok(error) => {
            format!("<p><i>{}</i></p>", htmlescape::encode_minimal(&error))
        }
        Err(e) => {
            tracing::warn! (
                error.message = %e,
                error.cause_chain = ?e,
                "Failed to verify query parameter using the hmac tag"
            );
            "".into()
        }
    };

    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(format!(
            r#"<!DOCTYPE html>
<html lang="en">
    <head>
        <meta http-equiv="content-type" content="text/html; charset=utf-8">
        <title>Login</title>
    </head>
    <body>
        {}
        <form action="/login" method="post">
            <label>Username
                <input
                    type="text"
                    placeholder="Enter Username"
                    name="username"
                >
            </label>
            <label>Password
                <input
                    type="password"
                    placeholder="Enter Password"
                    name="password"
                >
            </label>
            <button type="submit">Login</button>
        </form>
    </body>
</html>"#,
            error_html
        ))
}
