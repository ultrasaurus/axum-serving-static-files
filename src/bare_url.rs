use std::{
    borrow::Cow,
    path::{Path, PathBuf},
    task::{Context, Poll},
};

use axum::http::{Request, Response, Uri};
use tower_layer::Layer;
use tower_service::Service;

/// Layer that applies [`BareUrl`] which sanitizes paths.
///
/// See the [module docs](self) for more details.

#[derive(Clone, Debug)]
pub struct BareUrlLayer {
    local_dir: PathBuf
}

impl BareUrlLayer {
    pub fn new<P: AsRef<Path>>(path: P) -> Self{
        Self {
            local_dir: PathBuf::from(path.as_ref())
        }
    }
}

impl<S> Layer<S> for BareUrlLayer {
    type Service = BareUrl<S>;

    fn layer(&self, inner: S) -> Self::Service {
        println!("BareUrlLayer");
        BareUrl::setup_service(inner)
    }
}

/// Middleware to remove filesystem path traversals attempts from URL paths.
///
/// See the [module docs](self) for more details.
#[derive(Clone, Copy, Debug)]
pub struct BareUrl<S> {
    inner: S,
}

impl<S> BareUrl<S> {
    /// Setup given service so BareUrl will be called to fix URLs before calling it
    pub fn setup_service(inner: S) -> Self {
        Self { inner }
    }

    #[allow(unused)]
    /// Access the wrapped service.
    pub fn inner(&self) -> &S {
        &self.inner
    }
}

impl<S, ReqBody, ResBody> Service<Request<ReqBody>> for BareUrl<S>
where
    S: Service<Request<ReqBody>, Response = Response<ResBody>>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    #[inline]
    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<ReqBody>) -> Self::Future {
        println!("BareUrl call: {}", req.uri());
        sanitize_path(req.uri_mut());
        self.inner.call(req)
    }
}

fn sanitize_path(uri: &mut Uri) {
    let path_str = uri.path();
    let mut parts = uri.clone().into_parts();
    println!("sanitize_path: {}", path_str);
    let path = PathBuf::from(path_str);
    let ext = path.extension();
    if ext == None {
        let new_path = format!("{}.html", path_str);
        if let Some(path_and_query) = parts.path_and_query {
            let new_path_and_query = if let Some(query) = path_and_query.query() {
                Cow::Owned(format!("{new_path}?{query}"))
            } else {
                new_path.into()
            }
            .parse()
            .expect("url to still be valid");
            parts.path_and_query = Some(new_path_and_query);
            if let Ok(new_uri) = Uri::from_parts(parts) {
                *uri = new_uri;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;

    use tower::{ServiceBuilder, ServiceExt};

    use super::*;

    #[tokio::test]
    async fn layer() {
        async fn handle(request: Request<()>) -> Result<Response<String>, Infallible> {
            Ok(Response::new(request.uri().to_string()))
        }

        let mut svc = ServiceBuilder::new()
            .layer(BareUrlLayer)
            .service_fn(handle);

        let body = svc
            .ready()
            .await
            .unwrap()
            .call(Request::builder().uri("/../../secret").body(()).unwrap())
            .await
            .unwrap()
            .into_body();

        assert_eq!(body, "/secret");
    }

    #[test]
    fn no_path() {
        let mut uri = "/".parse().unwrap();
        sanitize_path(&mut uri);

        assert_eq!(uri, "/");
    }

    #[test]
    fn maintain_query() {
        let mut uri = "/?test".parse().unwrap();
        sanitize_path(&mut uri);

        assert_eq!(uri, "/?test");
    }

    #[test]
    fn path_maintain_query() {
        let mut uri = "/path?test=true".parse().unwrap();
        sanitize_path(&mut uri);

        assert_eq!(uri, "/path?test=true");
    }

    #[test]
    fn remove_path_parent_traversal() {
        let mut uri = "/../../path".parse().unwrap();
        sanitize_path(&mut uri);

        assert_eq!(uri, "/path");
    }

    #[test]
    fn remove_path_parent_traversal_maintain_query() {
        let mut uri = "/../../path?name=John".parse().unwrap();
        sanitize_path(&mut uri);

        assert_eq!(uri, "/path?name=John");
    }

    #[test]
    fn remove_path_current_traversal() {
        let mut uri = "/.././path".parse().unwrap();
        sanitize_path(&mut uri);

        assert_eq!(uri, "/path");
    }

    #[test]
    fn remove_path_encoded_traversal() {
        let mut uri = "/..%2f..%2fpath".parse().unwrap();
        sanitize_path(&mut uri);

        assert_eq!(uri, "/path");
    }

    #[test]
    fn keep_trailing_slash() {
        let mut uri = "/path/".parse().unwrap();
        sanitize_path(&mut uri);

        assert_eq!(uri, "/path/");
    }

    #[test]
    fn keep_only_one_trailing_slash() {
        let mut uri = "/path//".parse().unwrap();
        sanitize_path(&mut uri);

        assert_eq!(uri, "/path/");
    }
}