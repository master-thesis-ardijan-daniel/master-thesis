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

enum Node<T> {
    Leaf(LeafNode<T>),
    Parent(ParentNode<T>),
}

struct Coordinates {
    lat: f32,
    lon: f32,
}

struct LeafNode<T> {
    bounds: Coordinates,
    value: T,
}

struct ParentNode<T> {
    bounds: Coordinates,
    children: Vec<Node<T>>,
}

struct GeoTree<T> {
    root: Node<T>,
}

impl<T> GeoTree<T> {
    fn create<F>(input_data: Vec<T>, aggregation_function: F) -> Self
    where
        F: FnMut(Vec<T>) -> T,
    {

        let parent = aggregation_function()

        todo!()
    }
}
