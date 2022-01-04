//! tests/health_check.rs
use std::net::TcpListener;
// 'actix_rt::test' is the testing equivalent of 'actix_web::main'.
// It also spares you from having to specify the '#[test]' attribute/
//
// User ' cargo add actix-rt --dev --vers2' to add 'actix-rt'
// under '[dev-dependencies]' in Cargo.toml
//
// You can inspect what code gets generated using
// 'cargo expand --test health_check' (<- name of the test file)
#[actix_web::test]
async fn health_check_works(){
    // No .await, no .expect
    spawn_app();
    // We need to bring in 'request'
    // to perform HTTP requests against out application.
    //
    // User 'cargo add request --dev --vers 0.11' to add
    // it under '[dev-dependencies]' in Cargo.toml
    let client = reqwest::Client::new();

    // Act
    let response = client
        .get("http://127.0.0.1:8000/health_check")
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert
    assert!(response.status().is_success());
    assert_eq! (Some(0), response.content_length());
}

// No .await call, therefore no need for 'spawn_app' to be async now.
// We are also running tests, so it is not worth it to propagate errors:
// if we fail to perform the required setup we can jsut panic and crash
// all the things
fn spawn_app() {
    let listener = TcpListener::bind("127.0.0.1:0")
        .expect("Failed to bind random port");
    // We retrieve the port assigned to us by the OS
    let port = listener.local_addr().unwrap().port();
    let server = zero2prod::run(listener).expect("Failed to bind address");
    // Launch the server as a background task
    // tokio::spawn returns a handle to the spawned future,
    // but we have no use for it here, hence the non-binding let
    //wwwwwwwww
    // New dev dependency - let's add tokio to the party with
    // 'cargo add tokio --dev --vers 1'
    let _ = tokio::spawn(server);
}