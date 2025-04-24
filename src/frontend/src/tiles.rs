use common::{Bounds, TileRef};
use image::Pixel;
use wasm_bindgen::UnwrapThrowExt;

pub async fn get_tiles(area: Bounds) -> Vec<TileRef<[u8; 3]>> {
    let raw_data = gloo_net::http::Request::get("/tiles")
        .query([("level", "3")])
        .send()
        .await
        .expect("Error, request failed! ");

    raw_data
        .json()
        .await
        .expect("Unable to deserialize response, from tile request")
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Tile {
    nw_lat: f32,
    nw_lon: f32,
    se_lat: f32,
    se_lon: f32,
    data: Vec<[u8; 3]>,
}

impl TileUniform {}
