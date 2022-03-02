use actix_web::http::header::ContentType;
use actix_web::HttpResponse;

pub async fn change_password_form() -> Result<HttpResponse, actix_web::Error> {
    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta http-equiv="Content-Type" content="text/html; charset=utf-8" />
    <title>Change password </title>
</head>
<body>
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
</html>"#))
        }
