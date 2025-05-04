use axum::{
    Router, 
    routing::any_service,
};
use notify::Watcher;
use std::{net::SocketAddr, path::Path};
use tokio::net::TcpListener;
use tower_livereload::LiveReloadLayer;
use tower_http::services::{ServeDir,ServeFile};


#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let addr: SocketAddr = SocketAddr::from(([127, 0, 0, 1], 3030));

    let livereload = LiveReloadLayer::new();
    let reloader = livereload.reloader();

    let app = Router::new()
        .route("/{*key}", any_service(ServeDir::new("website")))
        .route("/", any_service(ServeFile::new("website/index.html")))
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