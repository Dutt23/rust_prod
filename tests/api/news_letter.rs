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

    let res = reqwest::Client::new()
        .post(format!("{}/newsletter", &app.address))
        .json(&news_letter_body)
        .send()
        .await
        .expect("Unable to send request");

    assert_eq!(res.status().as_u16(), 200);
}

#[tokio::test]
async fn news_letters_are_delivered_to_confirmed_customers() {
    let app = spawn_app().await;
    create_confirmed_customers(&app).await;

    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server);

    let news_letter_body = serde_json::json!({
        "title": "News letter title",
        "content" : {
            "text" : "Newsletter body as plain text",
            "html": "<p> Newsletter body as HTML <p>"
        }
    });

    let res = reqwest::Client::new()
        .post(format!("{}/newsletter", &app.address))
        .json(&news_letter_body)
        .send()
        .await
        .expect("Unable to send request");

    assert_eq!(res.status().as_u16(), 200);
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
