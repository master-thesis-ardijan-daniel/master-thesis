use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{HeaderValue, Response},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use backend::{deserialize::GeoTree, Bounds, Dataset};
use bytemuck::Pod;
use earth_map::EarthmapDataset;
use geo::Coord;
use serde::Deserialize;
use std::{net::SocketAddr, sync::Arc};
use tower_http::{services::ServeDir, trace::TraceLayer};

mod earth_map;
// mod light_pollution;
// mod population;

fn initialize_tree<P, F, D>(path: P, dataset: F) -> std::io::Result<GeoTree<D>>
where
    P: AsRef<std::path::Path>,
    F: Fn() -> D,
    D: Dataset,
    D::Type: Copy + Pod,
    D::AggregateType: Copy + Pod,
{
    if path.as_ref().try_exists()? {
        return backend::deserialize::GeoTree::new(path);
    }

    {
        let tree = backend::GeoTree::build(&dataset());
        tree.write_to_file(&path)?;
    }

    GeoTree::new(path)
}

#[tokio::main(flavor = "multi_thread", worker_threads = 16)]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let earth_map_tree = {
        let key = "EARTH_MAP_DATASET";
        let dataset = || {
            earth_map::EarthmapDataset::new(
                std::env::var(key).unwrap_or_else(|_| panic!("{key} environment variable")),
            )
        };

        Arc::new(initialize_tree("earth_map.db", dataset)?)
    };

    // let _population_tree = {
    //     let key = "POPULATION_DATASET";
    //     let dataset = || {
    //         population::PopulationDataset::new(
    //             std::env::var(key).unwrap_or_else(|_| panic!("{key} environment variable")),
    //         )
    //     };

    //     Arc::new(initialize_tree("population.db", dataset)?)
    // };

    // let _light_pollution_tree = {
    //     let key = "LIGHT_POLLUTION_DATASET";
    //     let dataset = || {
    //         LightPollutionDataset::new(
    //             std::env::var(key).unwrap_or_else(|_| panic!("{key} environment variable")),
    //         )
    //     };

    //     Arc::new(initialize_tree("light_pollution.db", dataset)?)
    // };

    let state = BackendState {
        earth_map_tree,
        // _population_tree,
        // _light_pollution_tree,
    };

    let addr = SocketAddr::from(([127, 0, 0, 1], 8000));

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    let router = Router::new()
        .fallback_service(ServeDir::new(env!("ASSETS_DIR")))
        .route("/tiles", get(get_tiles))
        .route("/tile/{z}/{y}/{x}", get(get_tile))
        .with_state(state);

    println!("Listening on {}:{}", addr.ip(), addr.port());

    axum::serve(listener, router.layer(TraceLayer::new_for_http())).await?;

    Ok(())
}

#[derive(Clone)]
struct BackendState {
    earth_map_tree: Arc<GeoTree<EarthmapDataset>>,
    // _population_tree: Arc<GeoTree<PopulationDataset>>,
    // _light_pollution_tree: Arc<GeoTree<LightPollutionDataset>>,
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

    Json(state.earth_map_tree.get_tiles(query, tile_query.level)).into_response()
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
    let data: Vec<u8> =
        bincode::serialize(&state.earth_map_tree.get_tile(x, y, z).unwrap()).unwrap();

    Response::builder()
        .header(
            "Cache-Control",
            HeaderValue::from_static("public, max-age=31536000, immutable"),
        )
        .body(Body::from(data))
        .unwrap()
}
