use dotenv::dotenv;
use once_cell::sync::Lazy;
use sqlx::{Connection, Executor, PgConnection, PgPool};

use uuid::Uuid;
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
                init_subscriber(get_subscriber(
                    subscriber_name,
                    default_filter_level,
                    std::io::stdout,
                ));
            } else {
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
    pub db_pool: PgPool,
}

pub async fn spawn_app() -> TestApp {
    dotenv().ok();
    // The first time `initialize` is invoked the code in `TRACING` is executed.
    // All other invocations will instead skip execution.
    Lazy::force(&TRACING);

    // Randomize configuration to ensure test isolation
    let configuration = {
        let mut c = get_configuration().expect("failed to read configuration.");
        // Use different database for each time
        c.database.database_name = Uuid::new_v4().to_string();
        // Use random port
        c.application.port = 0;
        c
    };

    // Create database and migrate the database
    configure_database(&configuration.database).await;

    // Launch the application as the background task
    // tokio::spawn returns a handle to the spawned future,
    let application = Application::build(configuration.clone())
        .await
        .expect("failed to build application");
    let address = format!("http://127.0.0.1:{}", application.port());
    let _ = tokio::spawn(application.run_until_stopped());

    TestApp {
        address,
        db_pool: get_connection_pool(&configuration.database),
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
