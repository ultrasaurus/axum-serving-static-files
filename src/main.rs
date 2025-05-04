use axum::{
    routing::get, 
    Router, Extension,
    extract::ConnectInfo,
    response::Html
};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tower_livereload::LiveReloadLayer;
use tower_http::services::ServeDir;
use tower::ServiceBuilder;

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

    let app = Router::new()
        .nest_service(
            "/", 
            ServiceBuilder::new()
                .service(ServeDir::new("website"))
        )
        .layer(LiveReloadLayer::new())
        .layer(Extension(config));
    
    let listener = TcpListener::bind(addr).await?;

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    ).await?;

    Ok(())
}