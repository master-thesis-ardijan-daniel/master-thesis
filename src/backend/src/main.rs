use axum::Router;
use image::Rgb;
use std::{net::SocketAddr, ops::Deref};
use tower_http::{services::ServeDir, trace::TraceLayer};

mod lib;
pub use lib::*;

// #[tokio::main]
// async fn main() {
//     let addr = SocketAddr::from(([127, 0, 0, 1], 8000));

//     let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

//     let router = Router::new().fallback_service(ServeDir::new(env!("ASSETS_DIR")));

//     axum::serve(listener, router.layer(TraceLayer::new_for_http()))
//         .await
//         .unwrap();
// }
//

fn main() {
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

    let tree = GeoTree::create::<EarthTextures>(data, bounds);

    // let tile = tree
    //     .get_tile(0, &Coordinate { lat: 0., lon: 0. })
    //     .expect("did not find tile at 0, 0");

    fn create_image(name: &str, input: &Tile<Pixel>) -> () {
        let height = input.len() as u32;
        let width = input[0].len() as u32;

        println!("{}, ({}, {})", name, width, height);

        let tile = input
            .into_iter()
            .flatten()
            .copied()
            .flatten()
            .collect::<Vec<_>>();

        let image = image::ImageBuffer::<Rgb<u8>, _>::from_raw(width, height, tile).unwrap();

        image.save(name).unwrap();
    }

    let mut x = 0;
    let mut y = 0;
    for tile in tree.root.children.iter().flatten() {
        let name = format!("tiles/{}_{}.png", x, y);

        create_image(&name, &tile.tile);

        x += 1;
        y += 1;
    }

    let mut x = 0;
    let mut y = 0;
    for tile in tree.root.children[0][0].children.iter().flatten() {
        let name = format!("tiles_child/{}_{}.png", x, y);

        create_image(&name, &tile.tile);

        x += 1;
        y += 1;
    }
    println!("total depth: {}", tree.depth);

    create_image("root.png", &tree.root.tile);

    let image = stitch_tiles(tree.root.children.clone());

    create_image("stitched_root.png", &image);
}
