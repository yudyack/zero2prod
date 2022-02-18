use dotenv::dotenv;
use once_cell::sync::Lazy;
use serde_json::Value;
use sqlx::{Connection, Executor, PgConnection, PgPool};

use uuid::Uuid;
use wiremock::MockServer;
use zero2prod::configuration::{get_configuration, DatabaseSettings};

use zero2prod::startup::{get_connection_pool, Application};
use zero2prod::telemetry::{
    get_line_subscriber, get_subscriber, init_subscriber,
};

// Ensure that the `tracing` stack is only initialised once using `once_cell`
static TRACING: Lazy<()> = Lazy::new(|| {
    let default_filter_level = "info".to_string();
    let subscriber_name = "test".to_string();
    // We cannot assign the output of `get_subscriber` to a variable based on the value of `TEST_LOG`
    // because the sink is part of the type returned by `get_subscriber`, therefore they are not the
    // same type. We could work around it, but this is the most straight-forward way of moving forward.

    match std::env::var("TEST_LOG") {
        Ok(v) => {
            if v == "json" {
                println!("Using JSON output");
                init_subscriber(get_subscriber(
                    subscriber_name,
                    default_filter_level,
                    std::io::stdout,
                ));
            } else {
                println!("Using text output");
                init_subscriber(get_line_subscriber(
                    default_filter_level,
                    std::io::stdout,
                ));
            }
        }
        _ => {
            let subscriber = get_subscriber(
                subscriber_name,
                default_filter_level,
                std::io::sink,
            );
            init_subscriber(subscriber);
        }
    };
});

pub struct TestApp {
    pub address: String,
    pub port: u16,
    pub db_pool: PgPool,
    pub email_server: MockServer,
}

pub struct ConfirmationLinks {
    pub html: reqwest::Url,
    pub plain_text: reqwest::Url,
}

impl TestApp {
    pub async fn post_subscriptions(&self, body: String) -> reqwest::Response {
        reqwest::Client::new()
            .post(&format!("{}/subscriptions", &self.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn post_newsletters(
        &self,
        body: &serde_json::Value,
    ) -> reqwest::Response {
        reqwest::Client::new()
            .post(&format!("{}/newsletters", &self.address))
            .json(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    // accept mock server request (json that will be send to postframe) and search and return links in request
    pub fn get_confirmation_links(
        &self,
        email_request: &wiremock::Request,
    ) -> ConfirmationLinks {
        // Parse the body as JSON, starting from raw bytes
        let body: Value = serde_json::from_slice(&email_request.body).unwrap();

        // Extract the link from one of the request fields
        let get_link = |s: &str| {
            let links: Vec<_> = linkify::LinkFinder::new()
                .links(s)
                .filter(|l| *l.kind() == linkify::LinkKind::Url)
                .collect();
            assert_eq!(links.len(), 1);
            let raw_link = links[0].as_str().to_owned();
            let mut confirmation_link = reqwest::Url::parse(&raw_link).unwrap();
            // make sure host is local
            assert_eq!(confirmation_link.host_str().unwrap(), "127.0.0.1");
            // make sure port is the same as the one used by the spawned app
            confirmation_link.set_port(Some(self.port)).unwrap();
            confirmation_link
        };

        let html = get_link(&body["HtmlBody"].as_str().unwrap());
        let plain_text = get_link(&body["TextBody"].as_str().unwrap());

        ConfirmationLinks { html, plain_text }
    }
}

pub async fn spawn_app() -> TestApp {
    dotenv().ok();
    // The first time `initialize` is invoked the code in `TRACING` is executed.
    // All other invocations will instead skip execution.
    Lazy::force(&TRACING);

    // Lauch a mock server to stand in for Postmark's API
    let email_server = MockServer::start().await;

    // Randomize configuration to ensure test isolation
    let configuration = {
        let mut c = get_configuration().expect("failed to read configuration.");
        // Use different database for each time
        c.database.database_name = Uuid::new_v4().to_string();
        // Use random port
        c.application.port = 0;
        // Use mock server as email API
        c.email_client.base_url = email_server.uri();
        c
    };

    // Create database and migrate the database
    configure_database(&configuration.database).await;

    // Launch the application as the background task
    // tokio::spawn returns a handle to the spawned future,
    let application = Application::build(configuration.clone())
        .await
        .expect("failed to build application");
    let application_port = application.port();
    let _ = tokio::spawn(application.run_until_stopped());

    TestApp {
        address: format!("http://127.0.0.1:{}", application_port),
        port: application_port,
        db_pool: get_connection_pool(&configuration.database),
        email_server,
    }
}

async fn configure_database(config: &DatabaseSettings) -> PgPool {
    // create database
    let mut connection = PgConnection::connect_with(&config.without_db())
        .await
        .expect("Failed to connect to Postgres");

    connection
        .execute(
            format!(r#"CREATE DATABASE "{}";"#, config.database_name).as_str(),
        )
        .await
        .expect("Failed to create database");

    let connection_pool = PgPool::connect_with(config.with_db())
        .await
        .expect("Failed connect to postres");

    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to migrate database");

    connection_pool
}
