use actix_session::storage::RedisSessionStore;
use actix_session::SessionMiddleware;
use actix_web::cookie::Key;
use actix_web::dev::Server;
use actix_web::web;
use actix_web::web::Data;
use actix_web::App;
use actix_web::HttpServer;
use actix_web_flash_messages::storage::CookieMessageStore;
use actix_web_flash_messages::FlashMessagesFramework;
use secrecy::ExposeSecret;
use secrecy::Secret;
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
use crate::routes::admin::admin_dashboard;
use crate::routes::admin::change_password;
use crate::routes::admin::change_password_form;
use crate::routes::admin::log_out;
use crate::routes::admin::publish_newsletter;
use crate::routes::health_check;
use crate::routes::home::home;
use crate::routes::login::login;
use crate::routes::login::login_form;
use crate::routes::subscribe;
use crate::routes::subscribe_confirm;

pub struct Application {
    port: u16,
    server: Server,
}

impl Application {
    // We have converted the 'build' function into a constructor for
    // the 'Application' struct.
    pub async fn build(configuration: Settings) -> Result<Self, anyhow::Error> {
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
            configuration.application.hmac_secret,
            configuration.redis_uri,
        )
        .await?;
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

pub async fn run(
    listener: TcpListener,
    connection_pool: PgPool,
    email_client: EmailClient,
    base_url: String,
    hmac_secret: Secret<String>,
    redis_uri: Secret<String>,
) -> Result<Server, anyhow::Error> {
    let connection_pool = web::Data::new(connection_pool);
    let email_client = Data::new(email_client);
    let base_url = Data::new(ApplicationBaseUrl(base_url));

    let secret_key = Key::from(hmac_secret.expose_secret().as_bytes());
    let message_store = CookieMessageStore::builder(secret_key.clone()).build();
    let message_framework =
        FlashMessagesFramework::builder(message_store).build();
    let redis_store = RedisSessionStore::new(redis_uri.expose_secret()).await?;
    let server = HttpServer::new(move || {
        App::new()
            .wrap(message_framework.clone())
            .wrap(SessionMiddleware::new(
                redis_store.clone(),
                secret_key.clone(),
            ))
            // Middlewares are added using the `wrap` method on `App`
            .wrap(TracingLogger::default())
            .route("/health_check", web::get().to(health_check))
            .route("/subscriptions", web::post().to(subscribe))
            .route("/subscriptions/confirm", web::get().to(subscribe_confirm))
            .route("/home", web::get().to(home))
            .route("/login", web::get().to(login_form))
            .route("/login", web::post().to(login))
            .route("/admin/dashboard", web::get().to(admin_dashboard))
            .route("/admin/password", web::get().to(change_password_form))
            .route("/admin/password", web::post().to(change_password))
            .route("/admin/logout", web::post().to(log_out))
            .route("/admin/newsletters", web::post().to(publish_newsletter))
            .app_data(connection_pool.clone())
            .app_data(email_client.clone())
            .app_data(base_url.clone())
            .app_data(Data::new(HmacSecret(hmac_secret.clone())))
    })
    .listen(listener)?
    .run();
    // No .await here
    Ok(server)
}

#[derive(Clone)]
pub struct HmacSecret(pub Secret<String>);
