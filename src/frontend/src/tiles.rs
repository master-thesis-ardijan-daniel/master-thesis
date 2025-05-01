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

#[derive(Debug)]
pub struct Tile {
    pub nw_lat: f32,
    pub nw_lon: f32,
    pub se_lat: f32,
    pub se_lon: f32,
    pub data: Vec<[u8; 3]>,
    pub width: u32,
    pub height: u32,
}

impl Into<TileMetadata> for &Tile {
    fn into(self) -> TileMetadata {
        TileMetadata {
            nw_lat: self.nw_lat,
            nw_lon: self.nw_lon,
            se_lat: self.se_lat,
            se_lon: self.se_lon,
            width: self.width,
            height: self.height,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TileMetadata {
    pub nw_lat: f32,
    pub nw_lon: f32,
    pub se_lat: f32,
    pub se_lon: f32,
    pub width: u32,
    pub height: u32,
}
