use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, HeaderValue},
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

mod population;
mod world;

#[tokio::main]
async fn main() {
    // let tree = {
    //     let data = world::EarthmapDataset::new("./8081_earthmap10k.jpg");
    //     GeoTree::build(&data)
    // };

    // let mut writer = std::fs::File::create("earth_map.db").unwrap();
    // tree.root.serialize(&mut writer).unwrap();
    // drop(writer);

    let population_tree = {
        let data = population::PopulationDataset::new(
            "/home/daniel/Nedlastinger/ppp_2020_1km_Aggregated.tif",
        );
        println!("Data read");
        GeoTree::build(&data)
    };
    println!("Built tree");

    let writer = std::fs::File::create("population.db").unwrap();
    let mut writer = AlignedWriter::new(writer);
    population_tree.root.serialize(&mut writer).unwrap();
    drop(writer);

    println!("total population: {:#?}", population_tree.root.aggregate);

    return;

    // let tree = backend::deserialize::GeoTree::new("test.db").unwrap();

    // let state = BackendState {
    //     image_tree: Arc::new(tree),
    // };

    let router = Router::new()
        .fallback_service(ServeDir::new(env!("ASSETS_DIR")))
        .route("/tiles", get(get_tiles))
        .route("/tile/{z}/{y}/{x}", get(get_tile))
        .with_state(state);
    // let addr = SocketAddr::from(([127, 0, 0, 1], 8000));

    // let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    // let router = Router::new()
    //     .fallback_service(ServeDir::new(env!("ASSETS_DIR")))
    //     .route("/tiles", get(get_tiles))
    //     .with_state(state);

    // println!("Listening on {}:{}", addr.ip(), addr.port());

    // axum::serve(listener, router.layer(TraceLayer::new_for_http()))
    //     .await
    //     .unwrap();
}

#[derive(Clone)]
struct BackendState {
    image_tree: Arc<backend::deserialize::GeoTree<EarthmapDataset>>,
}

#[derive(Deserialize)]
struct TilesQuery {
    level: u32,
}

async fn get_tiles(
    Query(tile_query): Query<TilesQuery>,
    State(state): State<BackendState>,
) -> impl IntoResponse {
    let query = Bounds::new(Coord { x: -180., y: 90. }, Coord { x: 180., y: -90. });

    Json(state.image_tree.get_tiles(query, tile_query.level)).into_response()
}

#[derive(Deserialize)]
struct TileQuery {
    x: usize,
    y: usize,
    z: usize,
}

async fn get_tile(
    Path(TileQuery { x, y, z }): Path<TileQuery>,
    State(state): State<BackendState>,
) -> impl IntoResponse {
    let mut headers = HeaderMap::new();

    headers.insert(
        "Cache-Control",
        HeaderValue::from_static("public, max-age=31536000, immutable"),
    );

    (headers, Json(state.image_tree.get_tile(x, y, z))).into_response()
}
