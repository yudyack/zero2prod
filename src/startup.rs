use actix_web::dev::Server;
use actix_web::web;
use actix_web::App;
use actix_web::HttpServer;
use sqlx::PgConnection;
use std::net::TcpListener;

use crate::routes::health_check;
use crate::routes::subscribe;

// we need to mark run as public
// it is no longer a binary entrypoint, therefore we can mark it as async
// without having to use any proc-macro incantation
pub fn run(
    listener: TcpListener,
    connection: PgConnection,
) -> Result<Server, std::io::Error> {
    let connection = web::Data::new(connection);
    let server = HttpServer::new( move || {
        App::new()
            .app_data(web::FormConfig::default())
            .route("/health_check", web::get().to(health_check))
            .route("/subscription", web::post().to(subscribe))
            .app_data(connection.clone())
    })
    .listen(listener)?
    .run();
    // No .await here
    Ok(server)
}
