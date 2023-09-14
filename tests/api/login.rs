use crate::helpers::{assert_is_redirected_to, spawn_app};

#[tokio::test]
async fn an_error_flash_message_is_set_on_failure() {
    let app = spawn_app().await;

    let login_body = serde_json::json!({
      "username": "random-username",
      "password": "random-password"
    });

    let response = app.post_login(&login_body).await;

    assert_is_redirected_to(&response, "/login");

    let login_html = app.get_login_html_helper().await;
    assert!(login_html.contains(r#"<p><i>Authentication failed</i></p>"#));

    // Reload page and check if message there.
    let login_html_2 = app.get_login_html_helper().await;
    assert!(!login_html_2.contains(r#"<p><i>Authentication failed</i></p>"#));
}

#[tokio::test]
async fn redirect_on_login_success() {
    let app = spawn_app().await;

    let login_body = serde_json::json!({
      "username": &app.test_user.username,
      "password": &app.test_user.password,
    });

    let response = app.post_login(&login_body).await;
    assert_is_redirected_to(&response, "/admin/dashboard");

    let admin_dash_html = app.get_admin_dashboard_html_helper().await;
    assert!(admin_dash_html.contains(&format!("Welcome {}", app.test_user.username)));
}
