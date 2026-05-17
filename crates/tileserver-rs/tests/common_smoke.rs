mod common;

#[tokio::test]
async fn empty_server_responds_to_health() {
    let server = common::empty_test_server();
    let response = server.get("/health").await;
    response.assert_status_ok();
    response.assert_text("OK");
}
