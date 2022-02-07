use crate::helpers::spawn_app;

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
    let app = spawn_app().await;
    // We need to bring in 'request'
    // to perform HTTP requests against out application.
    //
    // User 'cargo add request --dev --vers 0.11' to add
    // it under '[dev-dependencies]' in Cargo.toml
    let client = reqwest::Client::new();

    // Act
    let response = client
        .get(format!("{}/health_check", &app.address))
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert
    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}
