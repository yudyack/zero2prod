use crate::helpers::{spawn_app, ConfirmationLinks, TestApp};

use serde::Serialize;
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
    };

    let response = app.post_newsletters(&newsletter_request_body).await;

    // Assert
    assert_eq!(response.status().as_u16(), 200);
    // Mock verifies on Drop that we haven't sent the newsletter email
}

/// Use the public API of the application under test to create
/// an unconfirmed subscriber.
async fn create_unconfirmed_subscriber(app: &TestApp) -> ConfirmationLinks {
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

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

    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    // Act
    let newsletter_request_body = FormData {
        title: "Newsletter title".to_string(),
        text_content: "Newsletter body as plain text".to_string(),
        html_content: "<p>Newsletter body as HTML</p>".to_string(),
    };

    let response = app.post_newsletters(&newsletter_request_body).await;

    // Assert
    assert_eq!(response.status().as_u16(), 200);
    // Mock verifies on Drop that we haven't sent the newsletter email
}

#[tokio::test]
async fn newsletters_returns_400_for_invalid_data() {
    #[derive(Serialize)]
    struct NoTitle {
        text_content: String,
        html_content: String,
    }
    #[derive(Serialize)]
    struct NoContent {
        title: String,
    }

    // Arrange
    let app = spawn_app().await;
    let test_cases: Vec<(Box<dyn erased_serde::Serialize>, &str)> = vec![
        (
            {
                Box::new(NoTitle {
                    text_content: "Newsletter body as plain text".to_string(),
                    html_content: "<p>Newsletter body as HTML</p>".to_string(),
                })
            },
            "missing title",
        ),
        (
            {
                Box::new(NoContent {
                    title: "Newsletter title".to_string(),
                })
            },
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
async fn requests_missing_authorization_are_rejected() {
    // Arrange
    let app = spawn_app().await;

    let response = reqwest::Client::new()
        .post(&format!("{}/admin/newsletters", &app.address))
        .form(&FormData {
            title: "Newsletter title".to_string(),
            text_content: "Newsletter body as plain text".to_string(),
            html_content: "<p>Newsletter body as HTML</p>".to_string(),
        })
        .send()
        .await
        .expect("Failed execute request.");

    // Assert
    assert_eq!(401, response.status().as_u16());
    assert_eq!(
        r#"Basic realm="publish""#,
        response.headers()["WWW-Authenticate"]
    );
}

#[tokio::test]
async fn non_existing_user_is_rejected() {
    // Arrange
    let app = spawn_app().await;
    // Random credentials
    let username = Uuid::new_v4().to_string();
    let password = Uuid::new_v4().to_string();

    let respone = reqwest::Client::new()
        .post(&format!("{}/admin/newsletters", &app.address))
        .basic_auth(username, Some(password))
        .form(&FormData {
            title: "Newsletter title".to_string(),
            text_content: "Newsletter body as plain text".to_string(),
            html_content: "<p>Newsletter body as HTML</p>".to_string(),
        })
        .send()
        .await
        .expect("Failed execute request.");

    // Assert
    assert_eq!(401, respone.status().as_u16());
    assert_eq!(
        r#"Basic realm="publish""#,
        respone.headers()["WWW-Authenticate"]
    );
}

#[tokio::test]
async fn invalid_password_is_rejected() {
    // Arrange
    let app = spawn_app().await;
    // Random credentials
    let username = &app.test_user.username;
    let password = Uuid::new_v4().to_string();
    assert_ne!(app.test_user.password, password);

    let respone = reqwest::Client::new()
        .post(&format!("{}/admin/newsletters", &app.address))
        .basic_auth(username, Some(password))
        .form(&FormData {
            title: "Newsletter title".to_string(),
            text_content: "Newsletter body as plain text".to_string(),
            html_content: "<p>Newsletter body as HTML</p>".to_string(),
        })
        .send()
        .await
        .expect("Failed execute request.");

    // Assert
    assert_eq!(401, respone.status().as_u16());
    assert_eq!(
        r#"Basic realm="publish""#,
        respone.headers()["WWW-Authenticate"]
    );
}
