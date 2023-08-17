use crate::helpers::{spawn_app, ConfirmationLink, TestApp};
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
        "title": "News letter title",
        "content" : {
            "text" : "Newsletter body as plain text",
            "html": "<p> Newsletter body as HTML <p>"
        }
    });

    let res = app.post_news_letters(&news_letter_body).await;

    assert_eq!(res.status().as_u16(), 200);
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

    let news_letter_body = serde_json::json!({
        "title": "News letter title",
        "content" : {
            "text" : "Newsletter body as plain text",
            "html": "<p> Newsletter body as HTML <p>"
        }
    });

    let res = app.post_news_letters(&news_letter_body).await;

    assert_eq!(res.status().as_u16(), 200);
}

#[tokio::test]
async fn newsletter_returns_400_for_invalid_data() {
    let app = spawn_app().await;
    let test_cases = vec![
        (
            serde_json::json!({
                "content": {
                    "text" : "News letter text",
                    "html": "<p> Plain html body </p>"
                }
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
        "content" : {
            "text" : "Newsletter body as plain text",
            "html": "<p> Newsletter body as HTML <p>"
        }
    });

    let res = app.post_news_letters(&news_letter_body).await;

    assert_eq!(401, res.status().as_u16());
    assert_eq!(
        r#"Basic realm="publish""#,
        res.headers()["WWW-Authenticate"]
    )
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
