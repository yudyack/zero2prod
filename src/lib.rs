use actix_web::{App, HttpResponse, HttpServer, web};
use actix_web::dev::Server;
use std::net::TcpListener;


async fn health_check() -> HttpResponse {
    HttpResponse::Ok().finish()
}

// we need to mark run as public
// it is no longer a binary entrypoint, therefore we can mark it as async
// without having to use any proc-macro incantation
pub fn run(listener: TcpListener) -> Result<Server, std::io::Error> {
    let server = HttpServer::new(|| {
        App::new()
            .route("/health_check", web::get().to(health_check))
    })
        .listen(listener)?
        .run();
    // No .await here
    Ok(server)
}