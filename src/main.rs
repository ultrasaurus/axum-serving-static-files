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
    //let config = configuration::get_configuration()?;
    let config = Config {
        port: 3030
    };

    let addr = SocketAddr::from(([127, 0, 0, 1], config.port));

    let app = Router::new()
        .route("/", get(index))
        .nest_service(
            "/static", 
            ServiceBuilder::new()
                .service(ServeDir::new("static"))
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

async fn index(ConnectInfo(addr): ConnectInfo<SocketAddr>) -> Html<String> {
    let html = format!(
        "<h1>This is a test!</h1>\n\
         <img src=\"static/favicon.ico\"/>"
    );

    Html(html)
}