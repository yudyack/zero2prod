use actix_web::{http::header::ContentType, HttpResponse};
use actix_web_flash_messages::IncomingFlashMessages;
use std::fmt::Write;

use crate::{
    session_state::TypedSession,
    utils::{e500, see_other},
};

pub async fn newsletter_form(
    session: TypedSession,
    flash_messages: IncomingFlashMessages,
) -> Result<HttpResponse, actix_web::Error> {
    let idempotency_key = uuid::Uuid::new_v4();

    if session.get_user_id().map_err(e500)?.is_none() {
        return Ok(see_other("/login"));
    };

    let mut msg_html = String::new();
    for m in flash_messages.iter() {
        writeln!(msg_html, "<p><i>{}</i></p>", m.content()).unwrap();
    }

    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(format!(
            r#"<!DOCTYPE html>
            <html lang="en">
            
            <head>
                <meta http-equiv="content-type" content="text/html; charset=utf-8">
                <title>Newsletters</title>
            </head>
            
            <body>
                {msg_html}
                <form action="/admin/newsletters" method="post">
                    <label>Title
                        <input type="text" placeholder="Enter Title" name="title">
                    </label>
                    <br/>
                    <label>Text Content
                        <textarea name="text_content" placeholder="Your Email Body Here" required>
                        </textarea>
                    </label>
                    <br/>
                    <label>HTML Content
                        <textarea name="html_content" placeholder="Your Email Body Here" required>
                        </textarea>
                    </label>
                    <input hidden type="text" name="idempotency_key" value="{idempotency_key}">
                    <button type="submit">Post</button>
                </form>
            </body>
            </html>"#,
        )))
}
