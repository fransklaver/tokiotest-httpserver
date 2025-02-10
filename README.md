# tokiotest-httpserver [![CircleCI](https://circleci.com/gh/iroco-co/tokiotest-httpserver/tree/main.svg?style=svg&circle-token=a1da75459de58e46b72e0bdb2e41c7e65cdefadc)](https://circleci.com/gh/iroco-co/tokiotest-httpserver/tree/main)

A small test server utility to run http request against.

## parallel use

The test context instantiates a new server with a random port between 12300 and 12400. The test will use this port : 

```rust,no_run
use http_body_util::combinators::BoxBody;
use hyper::body::Bytes;
use hyper::{StatusCode, Uri};
use hyper_util::{
    client::legacy::{connect::HttpConnector, Client},
    rt::TokioExecutor,
};
use std::convert::Infallible;
use test_context::{test_context, AsyncTestContext};
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
```

At the end of the test, the port is released and can be used in another test.

## serial use

It is also possible to use it with a sequential workflow. You just have to include the [`serial_test`](https://docs.rs/serial_test) crate, and add the annotation.

With serial workflow you can choose to use a fixed port for the http test server by setting the environment variable `TOKIOTEST_HTTP_PORT` to the desired port.

See for example [test_serial](tests/test_serial.rs).
