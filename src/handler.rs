use crate::StatusCode;
use futures::future::BoxFuture;
use hyper::{Body, HeaderMap, Method, Request};
use std::convert::Infallible;
use std::sync::Arc;

pub type Response = hyper::Response<hyper::Body>;
pub type HandlerCallback = Arc<
    dyn Fn(Request<hyper::Body>) -> BoxFuture<'static, Result<Response, Infallible>> + Send + Sync,
>;

#[derive(Default, Clone)]
pub struct HandlerBuilder {
    path: String,
    method: Method,
    headers: HeaderMap,
    status_code: StatusCode,
}

#[allow(dead_code)]
impl HandlerBuilder {
    pub fn new(path: &str) -> HandlerBuilder {
        HandlerBuilder {
            path: String::from(path),
            method: Method::GET,
            headers: HeaderMap::new(),
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn method(mut self, method: Method) -> HandlerBuilder {
        self.method = method;
        self
    }

    pub fn status_code(mut self, status_code: StatusCode) -> HandlerBuilder {
        self.status_code = status_code;
        self
    }

    pub fn headers(mut self, headers: HeaderMap) -> HandlerBuilder {
        self.headers = headers;
        self
    }

    pub fn build(self) -> HandlerCallback {
        let Self {
            path,
            method,
            status_code,
            headers,
        } = self;
        Arc::new(move |req: Request<Body>| {
            let cloned_path = path.clone();
            let cloned_method = method.clone();
            let cloned_headers = headers.clone();
            Box::pin(async move {
                if req.uri().path().eq(cloned_path.as_str())
                    && req.method().eq(&cloned_method)
                    && Self::contains_headers(req.headers(), &cloned_headers)
                {
                    Ok(hyper::Response::builder()
                        .status(status_code)
                        .body(Body::empty())
                        .unwrap())
                } else {
                    Ok(hyper::Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .body(Body::empty())
                        .unwrap())
                }
            })
        })
    }

    fn contains_headers(
        headers_reference: &HeaderMap,
        headers_to_be_contained: &HeaderMap,
    ) -> bool {
        for (header, value) in headers_to_be_contained {
            if !headers_reference.get(header).eq(&Some(value)) {
                return false;
            }
        }
        true
    }
}

pub async fn default_handle(_req: Request<Body>) -> Result<Response, Infallible> {
    Ok(hyper::Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .body(Body::empty())
        .unwrap())
}
