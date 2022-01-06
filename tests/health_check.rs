//! tests/health_check.rs

use sqlx::{Connection, PgConnection};
use std::net::TcpListener;
use zero2prod::configuration::get_configuration;

// No .await call, therefore no need for 'spawn_app' to be async now.
// We are also running tests, so it is not worth it to propagate errors:
// if we fail to perform the required setup we can jsut panic and crash
// all the things
fn spawn_app() -> String {
    let listener =
        TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port");
    // We retrieve the port assigned to us by the OS
    let port = listener.local_addr().unwrap().port();
    let server =
        zero2prod::startup::run(listener).expect("Failed to bind address");
    // Launch the server as a background task
    // tokio::spawn returns a handle to the spawned future,
    // but we have no use for it here, hence the non-binding let
    //wwwwwwwww
    // New dev dependency - let's add tokio to the party with
    // 'cargo add tokio --dev --vers 1'
    let _ = tokio::spawn(server);
    format!("http://127.0.0.1:{}", port)
}

// 'actix_rt::test' is the testing equivalent of 'actix_web::main'.
// It also spares you from having to specify the '#[test]' attribute/
//
// User ' cargo add actix-rt --dev --vers2' to add 'actix-rt'
// under '[dev-dependencies]' in Cargo.toml
//
// You can inspect what code gets generated using
// 'cargo expand --test health_check' (<- name of the test file)
#[actix_web::test]
async fn health_check_works() {
    // No .await, no .expect
    let address = spawn_app();
    // We need to bring in 'request'
    // to perform HTTP requests against out application.
    //
    // User 'cargo add request --dev --vers 0.11' to add
    // it under '[dev-dependencies]' in Cargo.toml
    let client = reqwest::Client::new();

    // Act
    let response = client
        .get(format!("{}/health_check", address))
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert
    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}

#[actix_web::test]
async fn subscribe_return_a_200_for_valid_form_data() {
    // Arrange
    let app_address = spawn_app();
    let configuration =
        get_configuration().expect("Failed to get configuration");
    let connection_string = configuration.database.connection_string();
    // the 'Connection' trait must be in scope for us to invoke
    // 'PgConnection::connect' - it is not an inherent method of the struct!
    let mut connection = PgConnection::connect(&connection_string)
        .await
        .expect("Failed to connect to Postgres.");

    let client = reqwest::Client::new();
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    // Act
    let response = client
        .post(&format!("{}/subscription", app_address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert
    let saved = sqlx::query!("SELECT email, name FROM subscriptions",)
        .fetch_one(&mut connection)
        .await
        .expect("Failed to fetch saved subscription.");

    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");

    assert_eq!(200, response.status().as_u16());
}

#[actix_web::test]
async fn subscribe_return_a_400_when_data_is_missing() {
    // Arrange
    let app_address = spawn_app();
    let client = reqwest::Client::new();
    let test_cases = vec![
        ("name=le%20guin", "missing the email"),
        ("email=ursula_le_guin%40gmail.com", "missing the name"),
        ("", "missing both name and email"),
    ];

    // Act
    for (invalid_body, error_message) in test_cases {
        let response = client
            .post(&format!("{}/subscription", app_address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(invalid_body)
            .send()
            .await
            .expect("Failed to execute request.");

        // Assert
        assert_eq!(
            400,
            response.status().as_u16(),
            "the api did not fail with 400 bad request when the payload was {}",
            error_message
        );
    }
}
