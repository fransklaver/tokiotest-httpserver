use hyper::{Client, StatusCode, Uri};
use serial_test::serial;
use std::env;
use test_context::{test_context, AsyncTestContext};
use tokiotest_httpserver::handler::HandlerBuilder;
use tokiotest_httpserver::HttpTestContext;

#[test_context(PortContext)]
#[tokio::test]
#[serial]
async fn test_get_respond_200(ctx: &mut PortContext) {
    ctx.http_context.add(
        HandlerBuilder::new("/ok")
            .status_code(StatusCode::OK)
            .build(),
    );

    let resp = Client::new()
        .get(Uri::from_static("http://localhost:54321/ok"))
        .await
        .unwrap();

    assert_eq!(200, resp.status());
}

#[test_context(PortContext)]
#[tokio::test]
#[serial]
async fn test_get_respond_404(ctx: &mut PortContext) {
    ctx.http_context.add(
        HandlerBuilder::new("/notfound")
            .status_code(StatusCode::NOT_FOUND)
            .build(),
    );

    let resp = Client::new()
        .get(Uri::from_static("http://localhost:54321/notfound"))
        .await
        .unwrap();

    assert_eq!(404, resp.status());
}

#[allow(dead_code)]
struct PortContext {
    port_string: String,
    http_context: HttpTestContext,
}

#[async_trait::async_trait]
impl AsyncTestContext for PortContext {
    async fn setup() -> PortContext {
        let port_string = "54321".to_string();
        env::set_var("TOKIOTEST_HTTP_PORT", port_string.clone());
        PortContext {
            port_string,
            http_context: HttpTestContext::setup().await,
        }
    }

    async fn teardown(self) {
        self.http_context.teardown().await;
        env::remove_var("TOKIOTEST_HTTP_PORT");
    }
}
