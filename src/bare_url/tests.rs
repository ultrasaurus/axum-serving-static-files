use super::*;
use super::test_helpers::{
    to_bytes, 
    Body};
// use brotli::BrotliDecompress;
// use bytes::Bytes;
// use flate2::bufread::{DeflateDecoder, GzDecoder};
// use http::header::ALLOW;
// use http::{header, Method, Response};
use http::{Request, StatusCode};
use http_body::Body as HttpBody;
// use http_body_util::BodyExt;
// use std::convert::Infallible;
// use std::fs;
use tower::{
    // service_fn, 
    ServiceExt};

// use std::io::Read;

async fn body_into_text<B>(body: B) -> String
where
    B: HttpBody<Data = bytes::Bytes> + Unpin,
    B::Error: std::fmt::Debug,
{
    let bytes = to_bytes(body).await.unwrap();
    String::from_utf8(bytes.to_vec()).unwrap()
}

#[tokio::test]
async fn basic() {
    let svc = BareUrlServeDir::new("website");

    let req = Request::builder()
        .uri("/hello.html")
        .body(Body::empty())
        .unwrap();
    let res = svc.oneshot(req).await.unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.headers()["content-type"], "text/html");

    let body = body_into_text(res.into_body()).await;

    let contents = std::fs::read_to_string("./website/hello.html").unwrap();
    assert_eq!(body, contents);
}