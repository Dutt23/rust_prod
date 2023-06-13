use crate::helpers::spawn_app;

#[tokio::test]
async fn confirmation_without_token_reject_400() {
    let app = spawn_app().await;

    let response = reqwest::get(format!("{}/subscriptions/confirm", &app.address))
        .await
        .unwrap();

    assert_eq!(response.status().as_u16(), 400);
}
