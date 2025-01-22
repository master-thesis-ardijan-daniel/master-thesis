use axum::Router;
use std::net::SocketAddr;
use tower_http::{services::ServeDir, trace::TraceLayer};

#[tokio::main]
async fn main() {
    let addr = SocketAddr::from(([127, 0, 0, 1], 8000));

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    let router = Router::new().fallback_service(ServeDir::new(env!("ASSETS_DIR")));

    axum::serve(listener, router.layer(TraceLayer::new_for_http()))
        .await
        .unwrap();
}
