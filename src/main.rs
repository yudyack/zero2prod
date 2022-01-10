use sqlx::PgPool;
use std::net::TcpListener;
use tracing::subscriber::set_global_default;
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};

// this binary will target "package name"
use tracing_log::LogTracer;
use zero2prod::configuration::get_configuration;
use zero2prod::startup::run;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // We removed the `env_logger` line we had before!
    // env_logger::Builder::from_env(Env::default().default_filter_or("info"))
    //     .init();

    // Redirect all `log`'s events to our subscriber
    LogTracer::init().expect("Failed to set logger");

    // We are falling back to printing all spans at info-level or above
    // if the RUST_LOG environment variable has not been set.
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    let formatting_layer = BunyanFormattingLayer::new(
        "zero2prod".into(),
        // output the formatted to stdout
        std::io::stdout,
    );

    // The `with` method is provided by `SubscriberExt`, an extension
    // trait for `Subscriber` exposed by `tracing_subscriber`
    let subscriber = Registry::default()
        .with(env_filter)
        .with(JsonStorageLayer)
        .with(formatting_layer);
    // `set_global_default` can be used by applications to specify
    // what subscriber should be used to process spans.
    set_global_default(subscriber).expect("Failed to set subscriber");

    // panic if we can't read configuration file
    let configuration =
        get_configuration().expect("failed to read configuration");

    let connection_pool =
        PgPool::connect(&configuration.database.connection_string())
            .await
            .expect("Failed connect to postgres");

    let address = format!("127.0.0.1:{}", configuration.application_port);
    let listener = TcpListener::bind(address).expect("Failed to bind port");
    // We retrieve the port assigned to us by the OS
    // let port = listener.local_addr().unwrap().port();

    // Bubble up the io::Error if we failed to bind the address
    // Otherwise call .await on our Server
    run(listener, connection_pool)?.await
}
