use reqwest::Url;
use serde_json::Value;
use wiremock::{
    matchers::{method, path},
    Mock, ResponseTemplate,
};

use crate::helpers::spawn_app;

#[tokio::test]
async fn confirmation_without_token_are_rejected_with_a_400() {
    // Arrange
    let app = spawn_app().await;
    // Act
    let response =
        reqwest::get(&format!("{}/subscription/confirm", app.address))
            .await
            .unwrap();
    // Assert
    assert_eq!(response.status().as_u16(), 400);
}

#[tokio::test]
async fn the_link_returned_with_200_if_called() {
    // Arrange
    let app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin3%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        // We are not setting an expectation here anymore
        // The test is focused on another aspect of the app
        // behaviour.
        .mount(&app.email_server)
        .await;

    app.post_subscription(body.into()).await;
    let email_requests = &app.email_server.received_requests().await.unwrap();

    // Act
    // search and return links in json request that will be send to post frame
    let confirmation_link = app.get_confirmation_links(&email_requests[0]);
    // fire the link to spawned (test) app and get response
    let response = reqwest::get(confirmation_link.html).await.unwrap();

    // Assert
    assert_eq!(response.status().as_u16(), 200);
}


#[tokio::test]
async fn clicl_on_link_confirms_a_subscriber() {
    // Arrange
    let app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin3%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        // We are not setting an expectation here anymore
        // The test is focused on another aspect of the app
        // behaviour.
        .mount(&app.email_server)
        .await;

    app.post_subscription(body.into()).await;
    let email_requests = &app.email_server.received_requests().await.unwrap();

    // Act
    // search and return links in json request that will be send to post frame
    let confirmation_link = app.get_confirmation_links(&email_requests[0]);
    // fire the link to spawned (test) app and get response
    let response = reqwest::get(confirmation_link.html).await.unwrap();

    // Assert
    assert_eq!(response.status().as_u16(), 200);
}