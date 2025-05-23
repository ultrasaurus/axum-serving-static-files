use axum::{
    Router, 
    routing::any_service,
};
use notify::Watcher;
use std::{net::SocketAddr, path::Path};
use tokio::net::TcpListener;
use tower_livereload::LiveReloadLayer;
mod bare_url;
use bare_url::BareUrlServeDir;

const WEBSITE_DIR: &str = "website";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let addr: SocketAddr = SocketAddr::from(([127, 0, 0, 1], 3030));

    let livereload = LiveReloadLayer::new();
    let reloader = livereload.reloader();

    let app = Router::new()
        .fallback(any_service(BareUrlServeDir::new(WEBSITE_DIR)))
        .layer(livereload);

    let mut watcher = notify::recommended_watcher(move |_| reloader.reload())?;
    watcher.watch(Path::new("website"), notify::RecursiveMode::Recursive)?;

    
    let listener = TcpListener::bind(addr).await?;

    axum::serve(
        listener,
        app,
    ).await?;

    Ok(())
}