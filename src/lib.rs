#![doc = include_str!("../README.md")]
pub mod handler;

use crate::handler::{default_handle, HandlerCallback};
use hyper::service::service_fn;
use hyper::{StatusCode, Uri};
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server::graceful::GracefulShutdown;
use lazy_static::lazy_static;
use queues::{queue, IsQueue, Queue};
use std::collections::BinaryHeap;
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::Mutex;
use test_context::AsyncTestContext;
use tokio::net::TcpListener;
use tokio::select;
use tokio::sync::oneshot::{Receiver, Sender};
use tokio::task::JoinHandle;
use tokio::time::Duration;

pub type Error = Box<dyn std::error::Error + Send + Sync + 'static>;
pub static TOKIOTEST_HTTP_PORT_ENV: &str = "TOKIOTEST_HTTP_PORT";

lazy_static! {
    static ref PORTS: Mutex<BinaryHeap<u16>> =
        Mutex::new(BinaryHeap::from((12300u16..12400u16).collect::<Vec<u16>>()));
}

/// function that can be called to avoid port collision when tests have to open a listen port
/// this function takes a port in the `BinaryHeap PORTS`
pub fn take_port() -> u16 {
    PORTS.lock().unwrap().pop().unwrap()
}

/// function that can be called to release port after having taken it.
/// this function pushes the given port in the `BinaryHeap PORTS`
pub fn release_port(port: u16) {
    PORTS.lock().unwrap().push(port)
}

#[allow(dead_code)]
pub struct HttpTestContext {
    pub port: u16,
    pub handlers: Arc<Mutex<Queue<HandlerCallback>>>,
    server_handler: JoinHandle<Result<(), Error>>,
    sender: Sender<()>,
}

impl HttpTestContext {
    pub fn add(&mut self, handler: HandlerCallback) {
        self.handlers.lock().unwrap().add(handler).unwrap();
    }

    pub fn uri(&self, path: &str) -> Uri {
        format!("http://{}:{}{}", "localhost", self.port, path)
            .parse::<Uri>()
            .unwrap()
    }
}

pub async fn run_service(
    addr: SocketAddr,
    mut rx: Receiver<()>,
    handlers: Arc<Mutex<Queue<HandlerCallback>>>,
) -> Result<(), Error> {
    let graceful = GracefulShutdown::new();
    let listener = TcpListener::bind(addr).await?;
    loop {
        select! {
            conn = listener.accept() => {
                let (stream, _) = conn?;
                let io = TokioIo::new(stream);
                let cloned_handlers = handlers.clone();
                let builder = hyper_util::server::conn::auto::Builder::new(TokioExecutor::new());
                let conn = builder.serve_connection_with_upgrades(
                    io,
                    service_fn(move |req| match cloned_handlers.lock() {
                        Ok(mut handlers_rw) => match handlers_rw.remove() {
                            Ok(handler) => handler(req),
                            Err(_err) => Box::pin(default_handle(req)),
                        },
                        Err(_err_lock) => Box::pin(default_handle(req)),
                    }),
                );
                tokio::spawn(graceful.watch(conn.into_owned()));
            },
            _ = &mut rx => {
                drop(listener);
                break;
            }
        }
    }
    select! {
        _ = graceful.shutdown() => {
            Ok(())
        },
        _ = tokio::time::sleep(Duration::from_secs(10)) => {
            Err(Box::new(tokio::io::Error::new(
                tokio::io::ErrorKind::TimedOut,
                "Server graceful shutdown timed out",
            )))
        },
    }
}

impl AsyncTestContext for HttpTestContext {
    async fn setup() -> HttpTestContext {
        let port: u16 = match env::var(TOKIOTEST_HTTP_PORT_ENV) {
            Ok(port_str) => port_str.parse::<u16>().unwrap(),
            Err(_e) => take_port(),
        };
        let addr = SocketAddr::new("127.0.0.1".parse().unwrap(), port);
        let (sender, receiver) = tokio::sync::oneshot::channel::<()>();
        let handlers: Arc<Mutex<Queue<HandlerCallback>>> = Arc::new(Mutex::new(queue![]));
        let server_handler = tokio::spawn(run_service(addr, receiver, handlers.clone()));
        HttpTestContext {
            server_handler,
            sender,
            port,
            handlers,
        }
    }

    async fn teardown(self) {
        self.sender.send(()).unwrap();
        let _ = tokio::join!(self.server_handler);
        release_port(self.port);
    }
}

#[cfg(test)]
mod test {
    use std::convert::Infallible;

    use crate::handler::HandlerBuilder;
    use crate::HttpTestContext;
    use http_body_util::{combinators::BoxBody, Full};
    use hyper::{body::Bytes, HeaderMap, Method, Request, StatusCode};
    use hyper_util::{
        client::legacy::{connect::HttpConnector, Client},
        rt::TokioExecutor,
    };
    use test_context::test_context;

    macro_rules! make_client {
        () => {
            Client::builder(TokioExecutor::new())
                .build::<_, BoxBody<Bytes, Infallible>>(HttpConnector::new())
        };
    }

    #[test_context(HttpTestContext)]
    #[tokio::test]
    async fn test_get_without_expect_should_send_500(ctx: &mut HttpTestContext) {
        let resp = make_client!().get(ctx.uri("/whatever")).await.unwrap();
        assert_eq!(500, resp.status());
    }

    #[test_context(HttpTestContext)]
    #[tokio::test]
    async fn test_get_respond_404(ctx: &mut HttpTestContext) {
        ctx.add(
            HandlerBuilder::new("/unknown")
                .status_code(StatusCode::NOT_FOUND)
                .build(),
        );

        let resp = make_client!().get(ctx.uri("/unknown")).await.unwrap();

        assert_eq!(404, resp.status());
    }

    #[test_context(HttpTestContext)]
    #[tokio::test]
    async fn test_get_endpoint(ctx: &mut HttpTestContext) {
        ctx.add(
            HandlerBuilder::new("/foo")
                .status_code(StatusCode::OK)
                .build(),
        );

        let resp = make_client!().get(ctx.uri("/foo")).await.unwrap();
        assert_eq!(200, resp.status());

        let resp = make_client!().get(ctx.uri("/foo")).await.unwrap();
        assert_eq!(500, resp.status());
    }

    #[test_context(HttpTestContext)]
    #[tokio::test]
    async fn test_get_with_headers(ctx: &mut HttpTestContext) {
        let mut headers = HeaderMap::new();
        headers.append("foo", "bar".parse().unwrap());
        ctx.add(
            HandlerBuilder::new("/headers")
                .status_code(StatusCode::OK)
                .headers(headers.clone())
                .build(),
        );
        ctx.add(
            HandlerBuilder::new("/headers")
                .status_code(StatusCode::OK)
                .headers(headers)
                .build(),
        );

        let resp = make_client!().get(ctx.uri("/headers")).await.unwrap();
        assert_eq!(500, resp.status());

        let req = Request::builder()
            .method(Method::GET)
            .uri(ctx.uri("/headers"))
            .header("foo", "bar")
            .body(BoxBody::default())
            .unwrap();
        let resp = make_client!().request(req).await.unwrap();
        assert_eq!(200, resp.status());
    }

    #[test_context(HttpTestContext)]
    #[tokio::test]
    async fn test_post_endpoint(ctx: &mut HttpTestContext) {
        ctx.add(
            HandlerBuilder::new("/bar")
                .status_code(StatusCode::OK)
                .method(Method::POST)
                .build(),
        );

        let req = Request::builder()
            .method(Method::POST)
            .uri(ctx.uri("/bar"))
            .body(BoxBody::new(Full::new(Bytes::from("foo=bar"))))
            .expect("request builder");

        let resp = make_client!().request(req).await.unwrap();

        assert_eq!(200, resp.status());
    }
}
