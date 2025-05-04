use axum::{
    Router, Extension,
};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tower_livereload::LiveReloadLayer;
use tower_http::services::{ServeDir,ServeFile};

#[derive(Clone)]
struct Config {
    port: u16
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config {
        port: 3030
    };

    let addr: SocketAddr = SocketAddr::from(([127, 0, 0, 1], config.port));

    let serve_dir = ServeDir::new("website"); //.not_found_service(ServeFile::new("website/index.html"));

    let app = Router::new()
        .layer(LiveReloadLayer::new())
        .layer(Extension(config))
        .fallback_service(serve_dir);
    
    let listener = TcpListener::bind(addr).await?;

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    ).await?;

    Ok(())
}