use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{HeaderValue, Response},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use backend::{
    deserialize::GeoTree, earth_map::EarthmapDataset, light_pollution::LightPollutionDataset,
    population::PopulationDataset, Bounds, Dataset,
};
use bytemuck::Pod;
use geo::{Coord, Polygon};
use serde::Deserialize;
use std::{net::SocketAddr, sync::Arc};
use tower_http::{services::ServeDir, trace::TraceLayer};

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
            EarthmapDataset::new(
                std::env::var(key).unwrap_or_else(|_| panic!("{key} environment variable")),
            )
        };

        Arc::new(initialize_tree("earth_map.db", dataset)?)
    };

    // let population_tree = {
    //     let key = "POPULATION_DATASET";
    //     let dataset = || {
    //         PopulationDataset::new(
    //             std::env::var(key).unwrap_or_else(|_| panic!("{key} environment variable")),
    //         )
    //     };

    //     Arc::new(initialize_tree("population.db", dataset)?)
    // };

    let light_pollution_tree = {
        let key = "LIGHT_POLLUTION_DATASET";
        let dataset = || {
            LightPollutionDataset::new(
                std::env::var(key).unwrap_or_else(|_| panic!("{key} environment variable")),
            )
        };

        Arc::new(initialize_tree("light_pollution.db", dataset)?)
    };

    let state = BackendState {
        earth_map_tree,
        // population_tree,
        light_pollution_tree,
    };

    let addr = SocketAddr::from(([127, 0, 0, 1], 8000));

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    let router = Router::new()
        .fallback_service(ServeDir::new(env!("ASSETS_DIR")))
        .route("/tiles", get(get_tiles))
        .route("/sat_tile/{z}/{y}/{x}", get(get_tile))
        .route("/light_p_tile/{z}/{y}/{x}", get(get_lp_tile))
        // .route("/pop_tile/{z}/{y}/{x}", get(get_pop_tile))
        .route("/aggregate/lp", post(post_lp_aggregate))
        // .route("/aggregate/pop", post(post_pop_aggregate))
        .with_state(state);

    println!("Listening on {}:{}", addr.ip(), addr.port());

    axum::serve(listener, router.layer(TraceLayer::new_for_http())).await?;

    Ok(())
}

#[derive(Clone)]
struct BackendState {
    earth_map_tree: Arc<GeoTree<EarthmapDataset>>,
    // population_tree: Arc<GeoTree<PopulationDataset>>,
    light_pollution_tree: Arc<GeoTree<LightPollutionDataset>>,
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

// async fn get_pop_tile(
//     Path(TileQuery { x, y, z }): Path<TileQuery>,
//     State(state): State<BackendState>,
// ) -> impl IntoResponse {
//     let data: Vec<u8> =
//         bincode::serialize(&state.population_tree.get_tile(x, y, z).unwrap()).unwrap();

//     Response::builder()
//         .header(
//             "Cache-Control",
//             HeaderValue::from_static("public, max-age=31536000, immutable"),
//         )
//         .body(Body::from(data))
//         .unwrap()
// }

async fn get_lp_tile(
    Path(TileQuery { x, y, z }): Path<TileQuery>,
    State(state): State<BackendState>,
) -> impl IntoResponse {
    let data: Vec<u8> =
        bincode::serialize(&state.light_pollution_tree.get_tile(x, y, z).unwrap()).unwrap();

    Response::builder()
        .header(
            "Cache-Control",
            HeaderValue::from_static("public, max-age=31536000, immutable"),
        )
        .body(Body::from(data))
        .unwrap()
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

// async fn post_pop_aggregate(
//     State(state): State<BackendState>,
//     Json(query): Json<Polygon<f32>>,
// ) -> impl IntoResponse {
//     let aggregate = state.population_tree.get_aggregate(query);

//     Json(aggregate)
// }

async fn post_lp_aggregate(
    State(state): State<BackendState>,
    Json(query): Json<Polygon<f32>>,
) -> impl IntoResponse {
    let aggregate = state.light_pollution_tree.get_aggregate(query);

    Json(aggregate)
}

fn write_to_image() {
    
    use image::{ImageBuffer, Luma};

    let population_tree = {
        let key = "POPULATION_DATASET";
        let dataset = || {
            PopulationDataset::new(
                std::env::var(key).unwrap_or_else(|_| panic!("{key} environment variable")),
            )
        };

        initialize_tree("population.db", dataset).unwrap()
    };

    let level: usize = 3;
    let mut img_data = vec![];
    for y in 0..2_usize.pow(level as u32) {
        let mut row = vec![];
        for x in 0..2_usize.pow(level as u32) {
            row.push(
                population_tree
                    .get_tile(x, y, level)
                    .unwrap()
                    .data
                    .into_iter()
                    .map(|x| x.to_vec())
                    .collect::<Vec<_>>(),
            );
        }
        img_data.push(row);
    }

    let mut result = vec![];

    for outer_row in 0..img_data.len() {
        for inner_row in 0..img_data[0][0].len() {
            let mut row = Vec::new();

            for outer_col in 0..img_data[0].len() {
                row.extend_from_slice(&img_data[outer_row][outer_col][inner_row]);
            }

            result.push(row);
        }
    }

    let width = result[0].len();
    let height = result.len();

    let pixels = result
        .into_iter()
        .flatten()
        .map(|x| x as u16)
        .collect::<Vec<u16>>();

    let img: ImageBuffer<Luma<u16>, Vec<u16>> =
        ImageBuffer::from_vec(width as u32, height as u32, pixels)
            .ok_or("Failed to create image from normalized f32 data")
            .unwrap();

    // Save as TIFF
    img.save_with_format("testing_image.tiff", image::ImageFormat::Tiff)
        .unwrap();
}
