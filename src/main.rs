use std::net::TcpListener;

// this binary will target "package name"
use zero2prod::configuration::get_configuration;
use zero2prod::startup::run;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // panic if we can't read configuration file
    let configuration =
        get_configuration().expect("failed to read configuration");

    let address = format!("127.0.0.1:{}", configuration.application_port);
    let listener =
        TcpListener::bind(address).expect("Failed to bind port");
    // We retrieve the port assigned to us by the OS
    // let port = listener.local_addr().unwrap().port();

    // Bubble up the io::Error if we failed to bind the address
    // Otherwise call .await on our Server
    run(listener)?.await
}
