use crate::testapp::TestApp;

#[tokio::test]
async fn list_contract_with_success() {
    let app = TestApp::spawn().await;
    let resp = app
        .client
        .get(format!("{}/lcn", app.address))
        .send()
        .await
        .expect("failed to request contact list");

    assert_eq!(resp.status(), 200);
    let contracts = resp.json::<Vec<String>>().await.expect("unexpected response");
    assert!(!contracts.is_empty());
}