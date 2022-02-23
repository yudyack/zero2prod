use actix_web::dev::Server;
use actix_web::web;
use actix_web::web::Data;
use actix_web::App;
use actix_web::HttpServer;
use sqlx::migrate::MigrateError;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use sqlx::Pool;
use sqlx::Postgres;
use std::net::TcpListener;
use tracing_actix_web::TracingLogger;

use crate::configuration::DatabaseSettings;
use crate::configuration::Settings;
use crate::email_client::EmailClient;
use crate::routes::health_check;
use crate::routes::login_form;
use crate::routes::publish_newsletter;
use crate::routes::subscribe;
use crate::routes::subscribe_confirm;
use crate::routes::home;

pub struct Application {
    port: u16,
    server: Server,
}

impl Application {
    // We have converted the 'build' function into a constructor for
    // the 'Application' struct.
    pub async fn build(
        configuration: Settings,
    ) -> Result<Self, std::io::Error> {
        let connection_pool = get_connection_pool(&configuration.database);

        migrate(&configuration, &connection_pool)
            .await
            .expect("Failed to migrate database");

        let sender_email = configuration
            .email_client
            .sender()
            .expect("invalid sender email address");

        let timeout = configuration.email_client.timeout();

        let email_client = EmailClient::new(
            configuration.email_client.base_url,
            sender_email,
            configuration.email_client.authorization_token,
            timeout,
        );

        let address = format!(
            "{}:{}",
            configuration.application.host, configuration.application.port
        );
        tracing::info!("app started at: {}", &address);
        let listener = TcpListener::bind(address)?;
        let port = listener.local_addr().unwrap().port();
        let server = run(
            listener,
            connection_pool,
            email_client,
            configuration.application.base_url,
        )?;
        Ok(Self { port, server })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    // A more expressive name that makes it clear that
    // this function only returns when the application is stopped
    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        self.server.await
    }
}

pub fn get_connection_pool(configuration: &DatabaseSettings) -> PgPool {
    PgPoolOptions::new()
        .connect_timeout(std::time::Duration::from_secs(2))
        .connect_lazy_with(configuration.with_db())
}

async fn migrate(
    configuration: &Settings,
    connection_pool: &Pool<Postgres>,
) -> Result<(), MigrateError> {
    if configuration.database.migrate {
        tracing::info!("migrating postgres");
        sqlx::migrate!("./migrations").run(connection_pool).await
    } else {
        Ok(())
    }
}

// We need to define a wrapper type in order to retrieve the URL
// in the subscribe handler
// Retrieval from the context, in actix-web, is type based:
// using a raw String would expose us to conflictts
pub struct ApplicationBaseUrl(pub String);

pub fn run(
    listener: TcpListener,
    connection_pool: PgPool,
    email_client: EmailClient,
    base_url: String,
) -> Result<Server, std::io::Error> {
    let connection_pool = web::Data::new(connection_pool);
    let email_client = Data::new(email_client);
    let base_url = Data::new(ApplicationBaseUrl(base_url));
    let server = HttpServer::new(move || {
        App::new()
            // Middlewares are added using the `wrap` method on `App`
            .wrap(TracingLogger::default())
            .route("/health_check", web::get().to(health_check))
            .route("/subscriptions", web::post().to(subscribe))
            .route("/subscriptions/confirm", web::get().to(subscribe_confirm))
            .route("/newsletters", web::post().to(publish_newsletter))
            .route("/home", web::get().to(home))
            .route("/login", web::get().to(login_form))
            .app_data(connection_pool.clone())
            .app_data(email_client.clone())
            .app_data(base_url.clone())
    })
    .listen(listener)?
    .run();
    // No .await here
    Ok(server)
}
