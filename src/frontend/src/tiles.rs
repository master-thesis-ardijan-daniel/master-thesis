use common::{Bounds, TileRef};
use image::Pixel;
use wasm_bindgen::UnwrapThrowExt;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, Device, Extent3d, SamplerBindingType, SamplerDescriptor,
    ShaderStages, TextureDescriptor, TextureUsages, TextureViewDescriptor,
};

pub async fn get_tiles() -> Vec<TileRef<[u8; 4]>> {
    let raw_data = gloo_net::http::Request::get("/tiles")
        .query([("level", "2")])
        .send()
        .await
        .expect("Error, request failed! ");

    raw_data
        .json()
        .await
        .expect("Unable to deserialize response, from tile request")
}
