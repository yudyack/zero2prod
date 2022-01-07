use actix_web::web;
use actix_web::HttpResponse;
use sqlx::PgConnection;

#[allow(dead_code)]
#[derive(serde::Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}

pub async fn subscribe(
    _form: web::Form<FormData>,
    // retrieving from application state
    _connection: web::Data<PgConnection>,
) -> HttpResponse {
    HttpResponse::Ok().finish()
}
