use hyper::{Client, StatusCode};
use test_context::test_context;
use tokiotest_httpserver::handler::HandlerBuilder;
use tokiotest_httpserver::HttpTestContext;

#[test_context(HttpTestContext)]
#[tokio::test]
async fn test_get_respond_200(ctx: &mut HttpTestContext) {
    ctx.add(
        HandlerBuilder::new("/ok")
            .status_code(StatusCode::OK)
            .build(),
    );

    let resp = Client::new().get(ctx.uri("/ok")).await.unwrap();

    assert_eq!(200, resp.status());
}

#[test_context(HttpTestContext)]
#[tokio::test]
async fn test_get_respond_404(ctx: &mut HttpTestContext) {
    ctx.add(
        HandlerBuilder::new("/notfound")
            .status_code(StatusCode::NOT_FOUND)
            .build(),
    );

    let resp = Client::new().get(ctx.uri("/notfound")).await.unwrap();

    assert_eq!(404, resp.status());
}
