use std::{
    path::{Path, PathBuf},
    task::{Context, Poll},
};

use axum::http::{uri::PathAndQuery, Request, Response, Uri};
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
        BareUrl::setup_service(inner, &self.local_dir)
    }
}

/// Middleware to support "bare urls" (without .html extension)
#[derive(Clone, Debug)]
pub struct BareUrl<S> {
    inner: S,
    local_dir: PathBuf
}

impl<S> BareUrl<S> {
    /// Setup given service so BareUrl will be called to fix URLs before calling it
    pub fn setup_service<P: AsRef<Path>>(inner: S, path: P) -> Self {
        println!("BareUrl setup_service with local_dir: {}", path.as_ref().display());
        Self { inner, local_dir: PathBuf::from(path.as_ref()) }
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
        let mut chars = req.uri().path().chars();
        chars.next(); // remove initial '/' so its treated as relative path to local dir
        let path_str: &str = chars.as_str();

        let path = PathBuf::from(path_str);
        match path.extension() {
            None => {
                println!("no extension");
                println!("self.local_dir: {}",self.local_dir.display());
                let local_path = self.local_dir.join(path_str);
                println!("local_path: {}",local_path.display());
                if !local_path.exists() {
                    let alt_local_path = local_path.with_extension("html");
                    println!("alt local_path: {}",alt_local_path.display());
                    if alt_local_path.exists() {
                        let new_path_string = format!("{}.html", req.uri().path());
                        *req.uri_mut() = uri_with_path(req.uri(),&new_path_string);
                    }
                }
            },
            Some(ext) => {
                println!("extension = {:?}", ext);
                if ext == "html" {
                    // redirect
                }
            }
        }
        self.inner.call(req)
    }
}

fn uri_with_path(uri: &Uri, new_path_str: &str) -> Uri {
    let mut parts = uri.clone().into_parts();
    let new_path_and_query = 
        if let Some(query) = uri.query() {
            PathAndQuery::from_maybe_shared(format!("{new_path_str}?{query}"))
        } else {
            let path_bytes = new_path_str.to_string().as_bytes().to_owned();
            PathAndQuery::from_maybe_shared(path_bytes)
        }.expect("Uri to still be valid");
    parts.path_and_query = Some(new_path_and_query);
    Uri::from_parts(parts).expect("parts to be still valid")
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