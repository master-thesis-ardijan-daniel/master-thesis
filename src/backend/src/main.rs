use axum::{
    extract::{Query, State},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use image::Rgb;
use itertools::multizip;
use serde::Deserialize;
use std::{net::SocketAddr, ops::Deref, sync::Arc};
use tower_http::{services::ServeDir, trace::TraceLayer};

mod lib;
pub use lib::*;

#[tokio::main]
async fn main() {
    let addr = SocketAddr::from(([127, 0, 0, 1], 8000));

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    let state = BackendState {
        image_tree: Arc::new(create_tree()),
    };

    let router = Router::new()
        .fallback_service(ServeDir::new(env!("ASSETS_DIR")))
        .route("/tiles", get(get_tiles))
        .with_state(state);

    axum::serve(listener, router.layer(TraceLayer::new_for_http()))
        .await
        .unwrap();
}

fn create_tree() -> GeoTree<Pixel> {
    let image = image::open("./8081_earthmap10k.jpg").unwrap();

    let image = image.as_rgb8().unwrap();

    let mut data = Vec::new();
    for row in image.rows() {
        let mut out = Vec::new();
        for pixel in row {
            out.push(pixel.0);
        }
        data.push(out);
    }

    let bounds: Bounds = Bounds {
        north_west: Coordinate {
            lat: 90.,
            lon: -180.,
        },
        south_east: Coordinate {
            lat: -90.,
            lon: 180.,
        },
    };

    GeoTree::create::<EarthTextures>(data, bounds)
}

#[derive(Clone)]
struct BackendState {
    image_tree: Arc<GeoTree<Pixel>>,
}

#[derive(Deserialize)]
struct TileQuery {
    level: u32,
}

async fn get_tiles(
    Query(tile_query): Query<TileQuery>,
    State(state): State<BackendState>,
) -> impl IntoResponse {
    let query = Bounds {
        north_west: Coordinate {
            lat: 90.,
            lon: -180.,
        },
        south_east: Coordinate {
            lat: 85.,
            lon: -175.,
        },
    };

    Json(state.image_tree.get_tiles(query, tile_query.level)).into_response()
}

// fn main() {
//     let image = image::open("./8081_earthmap10k.jpg").unwrap();

//     let image = image.as_rgb8().unwrap();

//     let mut data = Vec::new();
//     for row in image.rows() {
//         let mut out = Vec::new();
//         for pixel in row {
//             out.push(pixel.0);
//         }
//         data.push(out);
//     }

//     let bounds: Bounds = Bounds {
//         north_west: Coordinate {
//             lat: 90.,
//             lon: -180.,
//         },
//         south_east: Coordinate {
//             lat: -90.,
//             lon: 180.,
//         },
//     };

//     let tree = GeoTree::create::<EarthTextures>(data, bounds);

//     {
//         let query = Bounds {
//             north_west: Coordinate {
//                 lat: 90.,
//                 lon: -180.,
//             },
//             south_east: Coordinate {
//                 lat: 85.,
//                 lon: -175.,
//             },
//         };

//         // assert!(query.intersects(&bounds));
//         // assert!(bounds.intersects(&query));

//         println!("Retrieving tiles");
//         dbg!(tree.root.bounds);
//         dbg!(tree.root.children.len());
//         dbg!(tree.root.children[0].len());
//         dbg!(tree.root.children[0][0].tile.len());
//         dbg!(tree.root.children[0][0].tile[0].len());
//         dbg!(tree.root.tile.len());
//         dbg!(tree.root.tile[0].len());
//         let result = tree.get_tiles(query, 2);
//         dbg!(result.len());
//         dbg!(result[0].len());
//         dbg!(result[0][0].len());
//     }

//     // let tile = tree
//     //     .get_tile(0, &Coordinate { lat: 0., lon: 0. })
//     //     .expect("did not find tile at 0, 0");

//     fn create_image(name: &str, input: &Tile<Pixel>) -> () {
//         let height = input.len() as u32;
//         let width = input[0].len() as u32;

//         println!("{}, ({}, {})", name, width, height);

//         let tile = input
//             .into_iter()
//             .flatten()
//             .copied()
//             .flatten()
//             .collect::<Vec<_>>();

//         let image = image::ImageBuffer::<Rgb<u8>, _>::from_raw(width, height, tile).unwrap();

//         image.save(name).unwrap();
//     }

//     let mut x = 0;
//     let mut y = 0;
//     for tile in tree.root.children.iter().flatten() {
//         let name = format!("tiles/{}_{}.png", x, y);

//         create_image(&name, &tile.tile);

//         x += 1;
//         y += 1;
//     }

//     let mut x = 0;
//     let mut y = 0;
//     dbg!(tree.root.children.len());
//     dbg!(tree.root.children[0].len());
//     dbg!(tree.root.children[0][0].children.len());
//     for tile in tree.root.children[0][0].children.iter().flatten() {
//         let name = format!("./tiles_child/{}_{}.png", x, y);

//         create_image(&name, &tile.tile);

//         x += 1;
//         y += 1;
//     }
//     println!("total depth: {}", tree.depth);

//     create_image("root.png", &tree.root.tile);

//     let image = stitch_tiles(tree.root.children.clone());

//     create_image("stitched_root.png", &image);
// }
