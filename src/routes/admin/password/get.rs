use actix_web::http::header::ContentType;
use actix_web::HttpResponse;
use actix_web_flash_messages::IncomingFlashMessages;
use std::fmt::Write;

use crate::session_state::TypedSession;
use crate::utils::{e500, see_other};

pub async fn change_password_form(
    session: TypedSession,
    flash_messages: IncomingFlashMessages,
) -> Result<HttpResponse, actix_web::Error> {
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
    <meta http-equiv="Content-Type" content="text/html; charset=utf-8" />
    <title>Change password </title>
</head>
<body>
{}
    <form action="/admin/password" method="post">
            <label>Current password
                <input
                    type="password"
                    placeholder="Current password"
                    name="current_password">
            </label>
            <br>
            <label>New password
                <input
                    type="password"
                    placeholder="New password"
                    name="new_password">
            </label>
            <br>
            <label>Confirm new password
                <input
                    type="password"
                    placeholder="Confirm new password"
                    name="confirm_new_password">
            </label>
            <br>
            <button type="submit">Change password</button>
    </form>
    <p><a href="/admin/dashboard">Back to dashboard</a></p>
</body>
</html>"#,
            msg_html
        )))
}