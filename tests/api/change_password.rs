use crate::helpers::{assert_is_redirected_to, spawn_app};
use uuid::Uuid;

#[tokio::test]
async fn you_must_be_logged_in_to_see_the_change_password_form() {
    let app = spawn_app().await;

    let response = app.change_password().await;

    assert_is_redirected_to(&response, "/login")
}

#[tokio::test]
async fn you_must_be_logged_in_to_change_your_password() {
    let app = spawn_app().await;
    let new_password = Uuid::new_v4().to_string();

    let response = app.post_password_change(&serde_json::json!({
      "current_password": Uuid::new_v4().to_string(), "new_password": &new_password, "new_password_check": &new_password,
      })).await;

    assert_is_redirected_to(&response, "/login");
}

#[tokio::test]
async fn new_password_fields_must_match() {
    let app = spawn_app().await;
    let password = Uuid::new_v4().to_string();
    let another_password = Uuid::new_v4().to_string();

    app.post_login(&serde_json::json!({
      "username": &app.test_user.username,
      "password": &app.test_user.password
    }))
    .await;

    let resp = app
        .post_password_change(&serde_json::json!({
          "current_password": &app.test_user.password,
          "new_password": &password,
          "new_password_check": &another_password
        }))
        .await;

    assert_is_redirected_to(&resp, "/admin/password");
    app.get_change_password_html().await.contains(
        "<p><i>You entered two different new passwords - \
    the field values must match.</i></p>",
    );
}
