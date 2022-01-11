use actix_web::dev::Server;
use actix_web::web;
use actix_web::App;
use actix_web::HttpServer;
use sqlx::PgPool;
use std::net::TcpListener;
use tracing_actix_web::TracingLogger;

use crate::routes::health_check;
use crate::routes::subscribe;

// we need to mark run as public
// it is no longer a binary entrypoint, therefore we can mark it as async
// without having to use any proc-macro incantation
pub fn run(
    listener: TcpListener,
    connection_pool: PgPool,
) -> Result<Server, std::io::Error> {
    let connection_pool = web::Data::new(connection_pool);
    let server = HttpServer::new(move || {
        App::new()
            // Middlewares are added using the `wrap` method on `App`
            .wrap(TracingLogger::default())
            .route("/health_check", web::get().to(health_check))
            .route("/subscription", web::post().to(subscribe))
            .app_data(connection_pool.clone())
    })
    .listen(listener)?
    .run();
    // No .await here
    Ok(server)
}
