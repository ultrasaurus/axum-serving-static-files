
use axum::http::{uri::PathAndQuery, Request, Response, Uri};
use bytes::Bytes;
use std::{
    convert::Infallible,
    future::Future,
    path::{Path, PathBuf},
    pin::Pin,
    task::{Context, Poll},
};
use tower_http::services::{ 
    fs::{DefaultServeDirFallback, ServeFileSystemResponseBody},
    ServeDir
};
use tower_service::Service;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod test_helpers;

/// Middleware to support "bare urls" (without .html extension)
#[derive(Clone, Debug)]
pub struct BareUrlServeDir<DefaultServeDirFallback> {
    inner: ServeDir,
    local_dir: PathBuf,
    #[allow(unused)]
    fallback: Option<DefaultServeDirFallback>
}

impl BareUrlServeDir<DefaultServeDirFallback> {
    /// Setup given service so BareUrl will be called to fix URLs before calling it
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        // let mut base = PathBuf::from(".");
        // base.push(path.as_ref());

        Self { 
            inner: ServeDir::new(path.as_ref()), 
            local_dir: PathBuf::from(path.as_ref()),
            fallback: None
        }
    }
}

impl<ReqBody, F, FResBody> Service<Request<ReqBody>> for BareUrlServeDir<F>
where
    ReqBody: Send + 'static,
    F: Service<Request<ReqBody>, Response = Response<FResBody>, Error = Infallible> + Clone,
    F::Future: Send + 'static,
    FResBody: http_body::Body<Data = Bytes> + Send + 'static,
    FResBody::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    type Response = Response<ServeFileSystemResponseBody>;
    type Error = Infallible;
    // type Future = InfallibleResponseFuture<ReqBody, F>;
    type Future = Pin<Box<dyn Future<Output = Result<Response<ServeFileSystemResponseBody>, Infallible>> +Send >>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        <ServeDir<DefaultServeDirFallback> as Service<Request<ReqBody>>>::poll_ready(&mut self.inner, cx)
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
            _ => {}
        }
        Box::pin(self.inner.call(req))
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

