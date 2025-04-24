use common::{Bounds, TileRef};
use image::Pixel;
use wasm_bindgen::UnwrapThrowExt;
use wgpu::{BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, Device, Extent3d, SamplerBindingType, SamplerDescriptor, ShaderStages, TextureDescriptor, TextureUsages, TextureViewDescriptor};

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
pub struct TileUniform {
    nw_lat: f32,
    nw_lon: f32,
    se_lat: f32,
    se_lon: f32,
    data: [[u8; 3]; 100_000],
    width: u32,
    height: u32,
}

impl TileUniform {}

pub struct TileState<T> {
    tile: TileRef<T>,
}

impl<T> TileState<T> {
    pub fn create(device: &Device, tile: TileRef<T>) -> Self {
        let size = Extent3d {
            width: tile.tile[0].len() as u32,
            height: tile.tile.len() as u32,
            depth_or_array_layers: 1,
        }
        
        let texture = device.create_texture(&TextureDescriptor {
            label: None,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Etc2Rgb8Unorm,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let texture_view = texture.create_view(&TextureViewDescriptor::default());
        let sampler = device.create_sampler(&SamplerDescriptor {
            label: None,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let texture_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: None,
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        count: None,
                    }
                ],
            });

        let texture_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &texture_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                }
            ],
        });

        todo!()
    }
}
