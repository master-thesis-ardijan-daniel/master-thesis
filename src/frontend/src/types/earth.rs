use common::{TileMetadata, TileRef};
use geo::{coord, Rect};
// use image::math::Rect;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroupEntry, Buffer, BufferAddress, BufferDescriptor, BufferUsages, Device, Extent3d,
    Origin3d, Queue, RenderPass, SamplerDescriptor, ShaderStages, TexelCopyBufferLayout,
    TexelCopyTextureInfo, TextureAspect, TextureDescriptor, TextureDimension, TextureFormat,
    TextureUsages, TextureViewDescriptor, VertexAttribute, VertexBufferLayout, VertexFormat,
    VertexStepMode,
};
use winit::event_loop::EventLoopProxy;

use crate::app::CustomEvent;

use super::{Icosphere, Point};

const TEXTURE_HEIGHT: u32 = 256;
const TEXTURE_WIDTH: u32 = TEXTURE_HEIGHT;

// const TEXTURE_ATLAS_SIZE: u32 = 2048;

#[derive(Debug)]
pub struct EarthState {
    vertex_buffer: Buffer,
    index_buffer: Buffer,

    icosphere: Icosphere,

    previous_subdivision_level: usize,
    current_subdivision_level: usize,
    previous_output_as_lines: bool,
    current_output_as_lines: bool,

    pub update_tile_buffer: bool,

    num_vertices: u32,
    num_indices: u32,
    texture_buffer: wgpu::Texture,
    texture_bind_group: wgpu::BindGroup,
    pub tiles: Vec<TileRef<[u8; 4]>>,
    pub texture_bind_group_layout: wgpu::BindGroupLayout,
    tile_metadata_buffer: Buffer,
    eventloop: EventLoopProxy<CustomEvent>,
    finished_creation: bool,
}

impl EarthState {
    // The shader code needs to loop over each of the tiles in order to check if any of them have anything it should sample
    // if so, we sample from the respective tile
    // need to make sure we sample withing width and height

    pub fn rewrite_tiles(&mut self, queue: &Queue) {
        let mut texture_data: Vec<u8> =
            Vec::with_capacity(self.tiles.len() * (TEXTURE_WIDTH * TEXTURE_HEIGHT * 4) as usize);

        for tile in &self.tiles {
            texture_data.extend(
                tile.get_padded_tile(TEXTURE_WIDTH, TEXTURE_HEIGHT)
                    .into_iter()
                    .flatten()
                    .flatten(),
            );
        }

        let tile_metadata = self
            .tiles
            .iter()
            .map(TileMetadata::from)
            .collect::<Vec<_>>();

        queue.write_buffer(
            &self.tile_metadata_buffer,
            0,
            bytemuck::cast_slice(&tile_metadata),
        );

        queue.write_texture(
            TexelCopyTextureInfo {
                texture: &self.texture_buffer,
                mip_level: 0,
                origin: Origin3d { x: 0, y: 0, z: 0 },
                aspect: TextureAspect::All,
            },
            &texture_data,
            TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(TEXTURE_WIDTH * 4),
                rows_per_image: Some(TEXTURE_HEIGHT),
            },
            Extent3d {
                width: TEXTURE_WIDTH,
                height: TEXTURE_HEIGHT,
                depth_or_array_layers: self.tiles.len() as u32,
            },
        );
    }

    pub fn write_a_single_tile_to_buffer(
        &mut self,
        new_tile: TileRef<[u8; 4]>,
        layer: u32,
        queue: Queue,
    ) {
        queue.write_texture(
            TexelCopyTextureInfo {
                texture: &self.texture_buffer,
                mip_level: 0,
                origin: Origin3d {
                    x: 0,
                    y: 0,
                    z: layer,
                },
                aspect: TextureAspect::All,
            },
            &new_tile
                .get_padded_tile(TEXTURE_WIDTH, TEXTURE_HEIGHT)
                .into_iter()
                .flatten()
                .flatten()
                .collect::<Vec<u8>>(),
            TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(TEXTURE_WIDTH * 4),
                rows_per_image: Some(TEXTURE_HEIGHT),
            },
            Extent3d {
                width: TEXTURE_WIDTH,
                height: TEXTURE_HEIGHT,
                depth_or_array_layers: 0,
            },
        );
    }

    pub fn fetch_tiles(&self, url: String) {
        // #[cfg(target_arch = "wasm32")]

        {
            let proxy = self.eventloop.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let raw_data = gloo_net::http::Request::get(&url)
                    .query([("level", "1")])
                    .send()
                    .await
                    .expect("Error, request failed! ");

                #[cfg(feature = "debug")]
                log::warn!("Tiles requested");

                let tiles = raw_data
                    .json()
                    .await
                    .expect("Unable to deserialize response, from tile request");
                proxy
                    .send_event(CustomEvent::HttpResponse(
                        crate::app::CustomResponseType::StartupTileResponse(tiles),
                    ))
                    .unwrap();
            });
        }
    }

    pub fn create(device: &Device, eventloop: EventLoopProxy<CustomEvent>) -> Self {
        let icosphere = Icosphere::new(1., Point::ZERO, 6, 0, vert_transform);

        let tile_metadata_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("tile_metadata_buffer"),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            size: size_of::<TileMetadata>() as u64 * 32,
            mapped_at_creation: false,
        });

        let texture_size = wgpu::Extent3d {
            width: TEXTURE_WIDTH,
            height: TEXTURE_HEIGHT,
            depth_or_array_layers: 32,
        };
        let texture_buffer = device.create_texture(&TextureDescriptor {
            label: Some("earth_texture_buffer"),
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8UnormSrgb,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let diffuse_texture_view = texture_buffer.create_view(&TextureViewDescriptor::default());
        let diffuse_sampler = device.create_sampler(&SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // Initializing empty buffers is fine,
        // since we initialize new ones on update
        let vertex_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("earth_vertex_buffer"),
            size: 0,
            usage: BufferUsages::VERTEX,
            mapped_at_creation: false,
        });

        let index_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("earth_index_buffer"),
            size: 0,
            usage: wgpu::BufferUsages::INDEX,
            mapped_at_creation: false,
        });

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("earth_texture_bind_group"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2Array,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("earth_texture_diffuse_bind_group"),
            layout: &texture_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&diffuse_texture_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&diffuse_sampler),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: tile_metadata_buffer.as_entire_binding(),
                },
            ],
        });

        let tiles = vec![TileRef {
            data: vec![vec![[125u8; 4]; 256]; 256],
            bounds: Rect::new(coord! { x: -180., y:90.}, coord! { x: 180., y:-90.}),
        }];

        Self {
            eventloop,
            vertex_buffer,
            index_buffer,
            previous_output_as_lines: false,
            current_output_as_lines: false,
            update_tile_buffer: true,
            finished_creation: false,
            texture_bind_group_layout,

            icosphere,
            previous_subdivision_level: 1,
            current_subdivision_level: 0,
            num_vertices: 0,
            num_indices: 0,
            texture_buffer,
            texture_bind_group,
            tile_metadata_buffer,
            tiles,
        }
    }

    pub fn descriptor() -> VertexBufferLayout<'static> {
        VertexBufferLayout {
            array_stride: std::mem::size_of::<[f32; 3]>() as BufferAddress,
            step_mode: VertexStepMode::Vertex,
            attributes: &[VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: VertexFormat::Float32x3,
            }],
        }
    }

    pub fn set_subdivision_level(&mut self, level: usize) {
        self.current_subdivision_level = level;
    }
    pub fn set_output_to_lines(&mut self, output_as_lines: bool) {
        self.current_output_as_lines = output_as_lines;
    }

    pub fn update(&mut self, queue: &Queue, device: &Device) {
        if !self.finished_creation {
            self.finished_creation = true;
            // the response handler will set self.update_tiles_buffer to true;
            self.fetch_tiles("/tiles".to_string());
        }

        if self.update_tile_buffer {
            self.rewrite_tiles(queue);
            self.update_tile_buffer = false;
        }

        if self.current_subdivision_level == self.previous_subdivision_level
            && self.previous_output_as_lines == self.current_output_as_lines
        {
            return;
        }

        let (icosphere_verts, icosphere_faces) = if self.current_output_as_lines {
            self.icosphere
                .get_subdivison_level_vertecies_and_lines(self.current_subdivision_level)
        } else {
            self.icosphere
                .get_subdivison_level_vertecies_and_faces(self.current_subdivision_level)
        };

        self.num_vertices = icosphere_verts.len() as u32;
        self.num_indices = icosphere_faces.len() as u32;

        let icosphere_verts = icosphere_verts
            .iter()
            .map(Point::to_array)
            .collect::<Vec<_>>();

        self.vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("earth_vertex_buffer"),
            contents: bytemuck::cast_slice(&icosphere_verts),
            usage: BufferUsages::VERTEX,
        });

        self.index_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("earth_index_buffer"),
            contents: bytemuck::cast_slice(&icosphere_faces),
            usage: BufferUsages::INDEX,
        });

        self.previous_subdivision_level = self.current_subdivision_level;
        self.previous_output_as_lines = self.current_output_as_lines;
    }

    pub fn render(&self, render_pass: &mut RenderPass<'_>) -> u32 {
        render_pass.set_bind_group(1, &self.texture_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

        self.num_indices
    }
}

fn vert_transform(mut v: Point) -> Point {
    v /= v.length();
    // const EARTH_RADIUS: 6378.137;
    const EARTH_RADIUS: f32 = 1.;
    // const FLATTENING: f32 = 1. / 298.257;
    const FLATTENING: f32 = 0.;
    // // get polar from cartesian
    let (lat, _lon, _rng) = v.to_lat_lon_range();
    // // get ellipsoid radius from polar
    let a = EARTH_RADIUS;
    let f = FLATTENING;
    let b = a * (1.0 - f);
    let sa = a * lat.sin();
    let cb = b * lat.cos();
    let r = a * b / (sa.powi(2) + cb.powi(2)).sqrt();
    // #[cfg(feature = "debug")]
    // {
    //     log::warn!("lat {:?}", lat * 180. / PI);
    //     log::warn!("lon {:?}", lon * 180. / PI);
    //     log::warn!("v {:?}", v);
    //     log::warn!("r {:?}", r);
    // }
    v * r
}
