use uuid::Uuid;

use crate::helpers::spawn_app;

#[tokio::test]
async fn you_must_be_logged_in_to_see_the_change_password_form() {
    // Arrange
    let app = spawn_app().await;

    // Act
    let response = app.get_change_password().await;

    // Assert
    assert_eq!(response.status().as_u16(), 303);
    assert_eq!(response.headers().get("Location").unwrap(), "/login");
}

#[tokio::test]
async fn you_must_be_logged_in_to_change_your_password() {
    // Arrange
    let app = spawn_app().await;
    let new_password = Uuid::new_v4().to_string();

    // Act
    let response = app
        .post_change_password(&serde_json::json!({
            "current_password" : Uuid::new_v4().to_string(),
            "new_password" : &new_password,
            "new_password_check" : &new_password,
        }))
        .await;

    // Assert
    assert_eq!(response.status().as_u16(), 303);
    assert_eq!(response.headers().get("Location").unwrap(), "/login");
}

#[tokio::test]
async fn new_password_fields_must_match() {
    // Arrange
    let app = spawn_app().await;
    let new_password = Uuid::new_v4().to_string();
    let another_new_passwrd = Uuid::new_v4().to_string();

    // Act - Part 1 - Login
    app.post_login(&serde_json::json!({
        "username" : &app.test_user.username,
        "password" : &app.test_user.password
    }))
    .await;

    // Act - Part 2 - Try to change password
    let response = app
        .post_change_password(&serde_json::json!({
            "current_password" : &app.test_user.password,
            "new_password" : &new_password,
            "new_password_check" : &another_new_passwrd,
        }))
        .await;

    assert_eq!(response.status().as_u16(), 303);
    assert_eq!(
        response.headers().get("Location").unwrap(),
        "/admin/password"
    );

    // Act - Part 3 - Follow the redirect
    let response = app.get_change_password().await;
    let html_page = response.text().await.unwrap();
    assert!(html_page.contains(
        "<p><i>You entered two different new passwords - \
        the field vallues must match.</i></p>"
    ));
}

#[tokio::test]
async fn current_password_must_be_valid() {
    // Arrange
    let app = spawn_app().await;
    let new_password = Uuid::new_v4().to_string();
    let wrong_password = Uuid::new_v4().to_string();

    // Act - Part 1 - Login
    app.post_login(&serde_json::json!({
        "username" : &app.test_user.username,
        "password" : &app.test_user.password
    }))
    .await;

    // Act - Part 2 - Try to change password
    let response = app
        .post_change_password(&serde_json::json!({
            "current_password" : &wrong_password,
            "new_password" : &new_password,
            "new_password_check" : &new_password,
        }))
        .await;

    assert_eq!(response.status().as_u16(), 303);
    assert_eq!(
        response.headers().get("Location").unwrap(),
        "/admin/password"
    );

    // Act - Part 3 - Follow the redirect
    let response = app.get_change_password().await;
    let html_page = response.text().await.unwrap();
    assert!(html_page.contains(
        "<p><i>The current password is incorrect.</i></p>"
    ));
}