use crate::helpers::spawn_app;
use reqwest::Url;
use wiremock::{
    matchers::{method, path},
    Mock, ResponseTemplate,
};

#[tokio::test]
async fn confirmation_without_token_reject_400() {
    let app = spawn_app().await;

    let response = reqwest::get(format!("{}/subscriptions/confirm", &app.address))
        .await
        .unwrap();

    assert_eq!(response.status().as_u16(), 400);
}

#[tokio::test]
async fn the_link_returned_by_subscribe_returns_a_200_if_called() {
    let app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body.into()).await;
    let email_request = &app.email_server.received_requests().await.unwrap()[0];

    let body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();

    let confirmation_link = app.get_links(&body["HtmlBody"].as_str().unwrap());
    let response = reqwest::get(format!("{}?subscription_token=mytoken", confirmation_link))
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 200);
}
