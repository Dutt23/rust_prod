use crate::helpers::{assert_is_redirected_to, spawn_app, ConfirmationLink, TestApp};
use actix_web_lab::test;
use wiremock::{
    matchers::{any, method, path},
    Mock, ResponseTemplate,
};

#[tokio::test]
async fn newsletters_are_not_delivered_to_unconfirmed_subscribers() {
    let app = spawn_app().await;
    create_unconfirmed_subscribers(&app).await;

    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&app.email_server)
        .await;

    let news_letter_body = serde_json::json!({
        "title": "Newsletter title",
        "text_content": "Newsletter body as plain text",
        "html_content": "<p>Newsletter body as HTML</p>",
        "idempotency_key": uuid::Uuid::new_v4().to_string(),
    });

    app.post_login(&serde_json::json!({
      "username": &app.test_user.username,
      "password": &app.test_user.password
    }))
    .await;

    let res = app.post_news_letters(&news_letter_body).await;

    assert_is_redirected_to(&res, "/admin/newsletters");
}

#[tokio::test]
async fn news_letters_are_delivered_to_confirmed_customers() {
    let app = spawn_app().await;
    create_confirmed_customers(&app).await;

    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    app.post_login(&serde_json::json!({
      "username": &app.test_user.username,
      "password": &app.test_user.password
    }))
    .await;

    let res = app
        .post_news_letters(&serde_json::json!({
            "title": "Newsletter title",
            "text_content": "Newsletter body as plain text",
            "html_content": "<p>Newsletter body as HTML</p>",
            "idempotency_key": uuid::Uuid::new_v4().to_string(),
        }))
        .await;

    assert_is_redirected_to(&res, "/admin/newsletters");
}

#[tokio::test]
async fn newsletter_returns_400_for_invalid_data() {
    let app = spawn_app().await;
    app.post_login(&serde_json::json!({
      "username": &app.test_user.username,
      "password": &app.test_user.password
    }))
    .await;
    let test_cases = vec![
        (
            serde_json::json!({
                    "text_content" : "News letter text",
                    "html_content": "<p> Plain html body </p>"

            }),
            "missing title",
        ),
        (
            serde_json::json!({"title": "News letter"}),
            "missing content",
        ),
    ];

    for (invalid_body, error_message) in test_cases {
        let res = app.post_news_letters(&invalid_body).await;

        assert_eq!(
            res.status().as_u16(),
            400,
            "Api did not fail when the error request body was {}",
            error_message
        );
    }
}

#[tokio::test]
async fn test_unauthorised_for_missing_credentials() {
    let app = spawn_app().await;
    create_confirmed_customers(&app).await;

    let news_letter_body = serde_json::json!({
           "title": "News letter title",
            "text_content" : "Newsletter body as plain text",
            "html_content": "<p> Newsletter body as HTML <p>"
    });

    let res = app.post_news_letters(&news_letter_body).await;

    dbg!(&res);
    assert_is_redirected_to(&res, "/login");
}

#[tokio::test]
async fn unknown_user_is_rejected() {
    let test_app = spawn_app().await;

    let res = test_app
        .post_news_letters(&serde_json::json!({
            "title": "News letter title",
                "text_content" : "Newsletter body as plain text",
                "html_content": "<p> Newsletter body as HTML <p>"
        }))
        .await;
    dbg!(&res);
    assert_is_redirected_to(&res, "/login");
}

async fn create_confirmed_customers(app: &TestApp) {
    let confirmation_link = create_unconfirmed_subscribers(&app).await;
    let res = reqwest::get(format!("{}", confirmation_link.html))
        .await
        .unwrap();

    assert_eq!(res.status().as_u16(), 200);
}

async fn create_unconfirmed_subscribers(app: &TestApp) -> ConfirmationLink {
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    let _mock_gaurd = Mock::given(path("/email"))
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

    let email_request = &app
        .email_server
        .received_requests()
        .await
        .unwrap()
        .pop()
        .unwrap();
    app.get_confirmation_links(email_request)
}

#[tokio::test]
async fn test_news_letter_creation_is_idempotent() {
    let app = spawn_app().await;
    create_confirmed_customers(&app).await;
    app.test_user.login(&app).await;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let newsletter_request_body = serde_json::json!({ "title": "Newsletter title",
    "text_content": "Newsletter body as plain text", "html_content": "<p>Newsletter body as HTML</p>", // We expect the idempotency key as part of the
    // form data, not as an header
    "idempotency_key": uuid::Uuid::new_v4().to_string()
        });

    let response = app.post_news_letters(&newsletter_request_body).await;
    assert_is_redirected_to(&response, "/admin/newsletters");

    let html_page = app.get_publish_newsletter().await;

    assert!(html_page.contains("<p><i>The newsletter issue has been published!</i></p>"));
    // Act - Part 3 - Submit newsletter form **again**
    let response = app.post_news_letters(&newsletter_request_body).await;
    assert_is_redirected_to(&response, "/admin/newsletters");
}
