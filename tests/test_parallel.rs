use http_body_util::combinators::BoxBody;
use hyper::body::Bytes;
use hyper::StatusCode;
use hyper_util::{
    client::legacy::{connect::HttpConnector, Client},
    rt::TokioExecutor,
};
use std::convert::Infallible;
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

    let resp = Client::builder(TokioExecutor::new())
        .build::<_, BoxBody<Bytes, Infallible>>(HttpConnector::new())
        .get(ctx.uri("/ok"))
        .await
        .unwrap();

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

    let resp = Client::builder(TokioExecutor::new())
        .build::<_, BoxBody<Bytes, Infallible>>(HttpConnector::new())
        .get(ctx.uri("/notfound"))
        .await
        .unwrap();

    assert_eq!(404, resp.status());
}
