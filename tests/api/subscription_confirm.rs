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
    let email_request = &app.email_server.received_requests().await.unwrap();
    // Get the first intercepted request
    // Parse the body as JSON, starting from raw bytes
    let body: Value = serde_json::from_slice(&email_request[0].body).unwrap();

    // Extract the link from one of the request fields
    let get_link = |s: &str| {
        let links: Vec<_> = linkify::LinkFinder::new()
            .links(s)
            .filter(|l| *l.kind() == linkify::LinkKind::Url)
            .collect();
        assert_eq!(links.len(), 1);
        links[0].as_str().to_owned()
    };

    let raw_confirmation_link = &get_link(&body["HtmlBody"].as_str().unwrap());
    let mut confirmation_link = Url::parse(raw_confirmation_link).unwrap();
    // make sure doesn't call random API on the web
    assert_eq!(confirmation_link.host_str().unwrap(), "127.0.0.1");

    // rewrite the url with port
    confirmation_link.set_port(Some(app.port)).unwrap();

    // Act
    let response = reqwest::get(confirmation_link).await.unwrap();

    // Assert
    assert_eq!(response.status().as_u16(), 200);
}
