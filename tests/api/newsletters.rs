use std::time::Duration;

use crate::helpers::{spawn_app, ConfirmationLinks, TestApp};

use crate::helpers::assert_is_redirect_to;
use fake::faker::internet::en::SafeEmail;
use fake::faker::name::en::Name;
use fake::Fake;
use uuid::Uuid;
use wiremock::matchers::{any, method, path};
use wiremock::{Mock, ResponseTemplate};
use zero2prod::routes::admin::newsletters::FormData;

#[tokio::test]
async fn you_must_be_logged_in_to_see_newsletters_form() {
    // Arrange
    let app = spawn_app().await;

    // Act
    let response = app.get_newsletters().await;

    // Assert
    assert_eq!(response.status().as_u16(), 303);
    assert_eq!(response.headers().get("Location").unwrap(), "/login");
}

#[tokio::test]
async fn newsletters_are_not_delivered_to_unconfirmed_subscribers() {
    // Arrange
    let app = spawn_app().await;
    create_unconfirmed_subscriber(&app).await;
    app.test_user.login(&app).await;

    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        // We assert that no request is fired at Postmark!
        .expect(0)
        .mount(&app.email_server)
        .await;

    // Act
    // A sketch of the newsletter payload structure.
    // We might change it later on.
    let newsletter_request_body = FormData {
        title: "Newsletter title".to_string(),
        text_content: "Newsletter body as plain text".to_string(),
        html_content: "<p>Newsletter body as HTML</p>".to_string(),
        idempotency_key: Uuid::new_v4().to_string(),
    };

    let response = app.post_newsletters(&newsletter_request_body).await;

    // Assert
    assert_is_redirect_to(&response, "/admin/newsletters");

    // Act Part 2 Follow the redirect
    let html_page = app.get_newsletters_html().await;
    assert!(html_page.contains(
        "The newsletter issue has been accepted - \
        emails will go out shortly."
    ));

    app.dispatch_all_pending_emails().await;
    // Mock verifies on Drop that we haven't sent the newsletter email
}

/// Use the public API of the application under test to create
/// an unconfirmed subscriber.
async fn create_unconfirmed_subscriber(app: &TestApp) -> ConfirmationLinks {
    // We are working with multiple subscribers now,
    // their details must be randomised to avoid conflicts
    let name: String = Name().fake();
    let email: String = SafeEmail().fake();
    let body = serde_urlencoded::to_string(&serde_json::json!({
        "name": name,
        "email": email,
    }))
    .unwrap();

    let _mock_guard = Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .named("Create unconfirmed subscriber")
        .expect(1)
        .mount_as_scoped(&app.email_server)
        .await;

    app.post_subscriptions(body.into())
        .await
        .error_for_status()
        .unwrap();

    // We now inspect the requests received by the mock Postmark server
    // to retrieve the confirmation link and return it
    let email_request = &app
        .email_server
        .received_requests()
        .await
        .unwrap()
        .pop()
        .unwrap();

    app.get_confirmation_links(&email_request)
}

async fn create_confirmed_subscriber(app: &TestApp) {
    // we can use the same helper and just add
    // an extra step to actually call the confirmation link!
    let confirmation_links = create_unconfirmed_subscriber(app).await;
    reqwest::get(confirmation_links.html)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
}

#[tokio::test]
async fn newsletters_are_delivered_to_confirmed_subscribers() {
    // Arrange
    let app = spawn_app().await;
    create_confirmed_subscriber(&app).await;
    // Login
    app.test_user.login(&app).await;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    // Act
    let newsletter_request_body = FormData {
        title: "Newsletter title".to_string(),
        text_content: "Newsletter body as plain text".to_string(),
        html_content: "<p>Newsletter body as HTML</p>".to_string(),
        idempotency_key: Uuid::new_v4().to_string(),
    };

    let response = app.post_newsletters(&newsletter_request_body).await;

    // Assert
    assert_is_redirect_to(&response, "/admin/newsletters");

    // Act 2 Follow the redirect
    let html_page = app.get_newsletters_html().await;
    assert!(html_page.contains(
        "The newsletter issue has been accepted - \
        emails will go out shortly."
    ));

    app.dispatch_all_pending_emails().await;
}

#[tokio::test]
async fn newsletters_returns_400_for_invalid_data() {
    // Arrange
    let app = spawn_app().await;
    app.test_user.login(&app).await;

    let test_cases = vec![
        (
            serde_json::json!({
                "html_content" : "<p>Newsletter body as HTML</p>",
                "text_content" : "Newsletter body as plain text",
            }),
            "missing title",
        ),
        (
            serde_json::json!({
                "title": "Newsletter title"
            }),
            "missing content",
        ),
    ];

    for (invalid_body, error_message) in test_cases {
        let response = app.post_newsletters(&invalid_body).await;

        // Assert
        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail with 400 Bad Request when the pylaod was {}.",
            error_message
        );
    }
}

#[tokio::test]
async fn you_must_be_logged_in_to_post_newsletter() {
    // Arrange
    let app = spawn_app().await;

    // Act
    let response = app
        .post_newsletters(&serde_json::json!({
            "title": "Newsletter title",
            "text_content": "Newsletter body as plain text",
            "html_content": "<p>Newsletter body as HTML</p>",
            "idempotency_key": Uuid::new_v4().to_string(),
        }))
        .await;

    // Assert
    assert_eq!(response.status().as_u16(), 303);
    assert_eq!(response.headers().get("Location").unwrap(), "/login");
}

#[tokio::test]
async fn you_must_be_logged_in_to_go_to_dashboard() {
    // Arrange
    let app = spawn_app().await;
    // app.test_user.login(&app).await;

    // Act
    let response = app.get_admin_dashboard().await;
    assert_is_redirect_to(&response, "/login");
}

#[tokio::test]
async fn newsletter_creation_is_idempotent() {
    //Arrange
    let app = spawn_app().await;
    create_confirmed_subscriber(&app).await;
    app.test_user.login(&app).await;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    // Act - Part 1 - Submit newsletter form
    let newsletter_request_body = FormData {
        title: "Newsletter title".to_string(),
        text_content: "Newsletter body as plain text".to_string(),
        html_content: "<p>Newsletter body as HTML</p>".to_string(),
        idempotency_key: Uuid::new_v4().to_string(),
    };
    let response = app.post_newsletters(&newsletter_request_body).await;
    assert_is_redirect_to(&response, "/admin/newsletters");

    // Act - Part 2 - Follow the redirect
    let html_page = app.get_newsletters_html().await;
    assert!(html_page.contains(
        "The newsletter issue has been accepted - \
        emails will go out shortly."
    ),);

    // Act - Part 3 - Submt newsletter form again
    let response = app.post_newsletters(&newsletter_request_body).await;
    assert_is_redirect_to(&response, "/admin/newsletters");

    // Act - Part 4 - Follow the redirect
    let html_page = app.get_newsletters_html().await;
    assert!(html_page.contains(
        "The newsletter issue has been accepted - \
        emails will go out shortly."
    ),);

    app.dispatch_all_pending_emails().await;
    // Mock verifies on Drop that we have sent the newsletter email once
}

#[tokio::test]
async fn concurrent_form_submisison_is_handled_gracefully() {
    //Arrange
    let app = spawn_app().await;
    create_confirmed_subscriber(&app).await;
    app.test_user.login(&app).await;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(
            ResponseTemplate::new(200).set_delay(Duration::from_secs(1)),
        )
        .expect(1)
        .mount(&app.email_server)
        .await;

    //  Submit newsletter form concurrently
    let newsletter_request_body = FormData {
        title: "Newsletter title".to_string(),
        text_content: "Newsletter body as plain text".to_string(),
        html_content: "<p>Newsletter body as HTML</p>".to_string(),
        idempotency_key: Uuid::new_v4().to_string(),
    };
    let response1 = app.post_newsletters(&newsletter_request_body);
    let response2 = app.post_newsletters(&newsletter_request_body);
    let (response1, response2) = tokio::join!(response1, response2);

    assert_eq!(response1.status().as_u16(), response2.status().as_u16());
    assert_eq!(
        response1.text().await.unwrap(),
        response2.text().await.unwrap()
    );

    app.dispatch_all_pending_emails().await;
    // Mock verifies on Drop that we have sent the newsletter email once
}
