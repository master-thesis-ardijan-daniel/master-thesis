use axum::{
    extract::{Query, State},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use backend::{
    deserialize::GeoTree,
    serialize::{AlignedWriter, Serialize as _},
    Bounds, Dataset,
};
use bytemuck::Pod;
use earth_map::EarthmapDataset;
use geo::Coord;
use light_pollution::LightPollutionDataset;
use population::PopulationDataset;
use serde::{de::Error, Deserialize};
use std::{net::SocketAddr, path::Path, sync::Arc};
use tower_http::{services::ServeDir, trace::TraceLayer};

mod earth_map;
mod light_pollution;
mod population;

fn initialize_tree<P, F, D>(path: P, dataset: F) -> std::io::Result<GeoTree<D>>
where
    P: AsRef<Path>,
    F: Fn() -> D,
    D: Dataset,
    D::Type: Copy + Pod,
    D::AggregateType: Copy + Pod,
{
    if path.as_ref().try_exists()? {
        return Ok(backend::deserialize::GeoTree::new(path)?);
    }

    {
        let tree = backend::GeoTree::build(&dataset());
        tree.write_to_file(&path)?;
    }

    GeoTree::new(path)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let earth_map_tree = {
        let key = "EARTH_MAP_DATASET";
        let dataset = || {
            earth_map::EarthmapDataset::new(
                std::env::var(key).expect(&format!("{key} environment variable")),
            )
        };

        Arc::new(initialize_tree("earth_map.db", dataset)?)
    };

    let population_tree = {
        let key = "POPULATION_DATASET";
        let dataset = || {
            population::PopulationDataset::new(
                std::env::var(key).expect(&format!("{key} environment variable")),
            )
        };

        Arc::new(initialize_tree("population.db", dataset)?)
    };

    let light_pollution_tree = {
        let key = "LIGHT_POLLUTION_DATASET";
        let dataset = || {
            LightPollutionDataset::new(
                std::env::var(key).expect(&format!("{key} environment variable")),
            )
        };

        Arc::new(initialize_tree("light_pollution.db", dataset)?)
    };

    let state = BackendState {
        earth_map_tree,
        population_tree,
        light_pollution_tree,
    };

    let addr = SocketAddr::from(([127, 0, 0, 1], 8000));

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    let router = Router::new()
        .fallback_service(ServeDir::new(env!("ASSETS_DIR")))
        .route("/tiles", get(get_tiles))
        .with_state(state);

    println!("Listening on {}:{}", addr.ip(), addr.port());

    axum::serve(listener, router.layer(TraceLayer::new_for_http())).await?;

    Ok(())
}

#[derive(Clone)]
struct BackendState {
    earth_map_tree: Arc<GeoTree<EarthmapDataset>>,
    population_tree: Arc<GeoTree<PopulationDataset>>,
    light_pollution_tree: Arc<GeoTree<LightPollutionDataset>>,
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

    Json(
        state
            .light_pollution_tree
            .get_tiles(query, tile_query.level),
    )
    .into_response()
}
