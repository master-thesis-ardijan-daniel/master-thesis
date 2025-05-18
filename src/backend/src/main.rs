use axum::{
    extract::{Query, State},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use backend::{
    serialize::{AlignedWriter, Serialize as _},
    Bounds, GeoTree,
};
use geo::Coord;
use serde::Deserialize;
use std::{net::SocketAddr, sync::Arc};
use tower_http::{services::ServeDir, trace::TraceLayer};
use world::EarthmapDataset;

mod world;

#[tokio::main]
async fn main() {
    let tree = {
        let data = world::EarthmapDataset::new("./8081_earthmap10k.jpg");
        GeoTree::build(&data)
    };

    let writer = std::fs::File::create("test.db").unwrap();
    let mut writer = AlignedWriter::new(writer);
    tree.root.serialize(&mut writer).unwrap();
    drop(writer);

    let tree = backend::deserialize::GeoTree::new("test.db").unwrap();

    let state = BackendState {
        image_tree: Arc::new(tree),
    };

    let addr = SocketAddr::from(([127, 0, 0, 1], 8000));

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    let router = Router::new()
        .fallback_service(ServeDir::new(env!("ASSETS_DIR")))
        .route("/tiles", get(get_tiles))
        .with_state(state);

    println!("Listening on {}:{}", addr.ip(), addr.port());

    axum::serve(listener, router.layer(TraceLayer::new_for_http()))
        .await
        .unwrap();
}

#[derive(Clone)]
struct BackendState {
    image_tree: Arc<backend::deserialize::GeoTree<EarthmapDataset>>,
}

#[derive(Deserialize)]
struct TileQuery {
    level: u32,
}

async fn get_tiles(
    Query(tile_query): Query<TileQuery>,
    State(state): State<BackendState>,
) -> impl IntoResponse {
    let query = Bounds::new(Coord { x: -180., y: 90. }, Coord { x: 180., y: -90. });

    Json(state.image_tree.get_tiles(query, tile_query.level)).into_response()
}
