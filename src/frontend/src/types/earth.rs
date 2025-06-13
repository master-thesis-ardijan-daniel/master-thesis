use std::collections::HashMap;

use web_time::Instant;

use common::{Bounds, TileMetadata, TileResponse};
use geo::{coord, Coord, Rect};
use glam::{Quat, Vec3};
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroupEntry, Buffer, BufferAddress, BufferDescriptor, BufferUsages, Device, Extent3d,
    Origin3d, Queue, RenderPass, SamplerDescriptor, ShaderStages, TexelCopyBufferLayout,
    TexelCopyTextureInfo, TextureAspect, TextureDescriptor, TextureDimension, TextureFormat,
    TextureUsages, TextureViewDescriptor, VertexAttribute, VertexBufferLayout, VertexFormat,
    VertexStepMode,
};
use winit::event_loop::EventLoopProxy;

use crate::{
    app::CustomEvent,
    camera::{Camera, Projection},
    utils::buffer::{BufferAllocator, BufferSlot, Level},
};

use super::Icosphere;

type Point = Vec3;

const TEXTURE_HEIGHT: u32 = 256;
const TEXTURE_WIDTH: u32 = TEXTURE_HEIGHT;
const BUFFER_SIZE: u32 = 256;

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
    pub texture_bind_group_layout: wgpu::BindGroupLayout,
    tile_metadata_buffer: Buffer,
    eventloop: EventLoopProxy<CustomEvent>,

    buffer_allocator: BufferAllocator,
    tile_map: HashMap<(u32, u32, u32), TileResponse<[u8; 4]>>,
    // population_buffer_allocator: BufferAllocator,
    // population_tile_map: HashMap<(u32, u32, u32), TileResponse<f32>>,
    lp_tile_map: HashMap<(u32, u32, u32), TileResponse<f32>>,
    lp_buffer_allocator: BufferAllocator,
    texture_buffer_2: wgpu::Texture,
    tile_metadata_buffer_2: Buffer,
    last_buffer_write: Instant,
    pub render_lp_map: bool,
    shader_mode_uniform: Buffer,
    shader_mode: u32,
    // pub query_poi: QueryPoi,
}

impl EarthState {
    pub fn insert_tile(&mut self, id: (u32, u32, u32), data: TileResponse<[u8; 4]>) {
        self.tile_map.insert(id, data);
    }

    // pub fn insert_population_tile(&mut self, id: (u32, u32, u32), data: TileResponse<f32>) {
    // self.population_tile_map.insert(id, data);
    // }

    pub fn insert_lp_tile(&mut self, id: (u32, u32, u32), data: TileResponse<f32>) {
        self.lp_tile_map.insert(id, data);
    }

    pub fn rewrite_tiles(&mut self, queue: &Queue) {
        if self.last_buffer_write.elapsed().as_millis() < 10 {
            return;
        }

        self.last_buffer_write = Instant::now();

        self.update_tile_buffer = false;
        let tiles = std::mem::take(&mut self.tile_map);

        for (id, tile) in tiles.into_iter() {
            let Some(&slot) = self.buffer_allocator.slot(&id) else {
                continue;
            };

            let data = tile
                .get_padded_tile(TEXTURE_WIDTH, TEXTURE_HEIGHT)
                .into_iter()
                .flatten()
                .flatten()
                .collect::<Vec<u8>>();
            let metadata = TileMetadata::from((&tile, id.0, 0));

            self.write_a_single_tile_to_buffer(&data, metadata, slot, queue);
        }

        // let tiles = std::mem::take(&mut self.population_tile_map);

        // for (id, tile) in tiles.into_iter() {
        //     let Some(&slot) = self.population_buffer_allocator.slot(&id) else {
        //         continue;
        //     };

        //     let data = tile
        //         .get_padded_tile(TEXTURE_WIDTH, TEXTURE_HEIGHT)
        //         .into_iter()
        //         .flatten()
        //         .flat_map(|pixel| pixel.to_ne_bytes())
        //         .collect::<Vec<u8>>();
        //     let metadata = TileMetadata::from((&tile, id.0, 1));

        //     self.write_a_single_tile_to_buffer(&data, metadata, slot, queue);
        // }

        let tiles = std::mem::take(&mut self.lp_tile_map);

        for (id, tile) in tiles.into_iter() {
            let Some(&slot) = self.lp_buffer_allocator.slot(&id) else {
                continue;
            };

            let data = tile
                .get_padded_tile(TEXTURE_WIDTH, TEXTURE_HEIGHT)
                .into_iter()
                .flatten()
                .flat_map(|pixel| pixel.to_ne_bytes())
                .collect::<Vec<u8>>();
            let metadata = TileMetadata::from((&tile, id.0, 2));

            self.write_a_single_tile_to_buffer(&data, metadata, slot, queue);
        }
    }

    pub fn write_a_single_tile_to_buffer(
        &mut self,
        data: &[u8],
        metadata: TileMetadata,
        slot: BufferSlot,
        queue: &Queue,
    ) {
        // let metadata = TileMetadata::from((&new_tile, id.0));
        let (texture_buffer, tile_metadata_buffer) = if metadata.data_type == 2 {
            (&self.texture_buffer_2, &self.tile_metadata_buffer_2)
        } else {
            (&self.texture_buffer, &self.tile_metadata_buffer)
        };
        queue.write_texture(
            TexelCopyTextureInfo {
                texture: texture_buffer,
                mip_level: 0,
                origin: Origin3d {
                    x: 0,
                    y: 0,
                    z: *slot as u32,
                },
                aspect: TextureAspect::All,
            },
            data,
            TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(TEXTURE_WIDTH * 4),
                rows_per_image: Some(TEXTURE_HEIGHT),
            },
            Extent3d {
                width: TEXTURE_WIDTH,
                height: TEXTURE_HEIGHT,
                depth_or_array_layers: 1,
            },
        );

        queue.write_buffer(
            tile_metadata_buffer,
            ({ *slot } * size_of::<TileMetadata>()) as u64,
            bytemuck::bytes_of(&metadata),
        );
    }

    pub fn create(device: &Device, eventloop: EventLoopProxy<CustomEvent>) -> Self {
        let icosphere = Icosphere::new(1., Point::ZERO, 6, 0, icosahedron_to_wgs84);

        let tile_metadata_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("tile_metadata_buffer"),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            size: size_of::<TileMetadata>() as u64 * BUFFER_SIZE as u64,
            mapped_at_creation: false,
        });

        let tile_metadata_buffer_2 = device.create_buffer(&BufferDescriptor {
            label: Some("tile_metadata_buffer"),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            size: size_of::<TileMetadata>() as u64 * BUFFER_SIZE as u64,
            mapped_at_creation: false,
        });

        let shader_mode_uniform = device.create_buffer(&BufferDescriptor {
            label: Some("shader mode"),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            size: size_of::<[u32; 4]>() as u64 * 4,
            mapped_at_creation: false,
        });

        let texture_size = wgpu::Extent3d {
            width: TEXTURE_WIDTH,
            height: TEXTURE_HEIGHT,
            depth_or_array_layers: BUFFER_SIZE,
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
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let texture_buffer_2 = device.create_texture(&TextureDescriptor {
            label: Some("earth_texture_buffer"),
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8UnormSrgb,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let diffuse_texture_view_2 =
            texture_buffer_2.create_view(&TextureViewDescriptor::default());
        let diffuse_sampler_2 = device.create_sampler(&SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
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
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                            view_dimension: wgpu::TextureViewDimension::D2Array,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                            view_dimension: wgpu::TextureViewDimension::D2Array,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 5,
                        visibility: ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 6,
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
                    resource: wgpu::BindingResource::TextureView(&diffuse_texture_view_2),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&diffuse_sampler_2),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: tile_metadata_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 5,
                    resource: tile_metadata_buffer_2.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 6,
                    resource: shader_mode_uniform.as_entire_binding(),
                },
            ],
        });

        let buffer_allocator = {
            let levels = (0..=7)
                .map(|level| {
                    Level::new(
                        Bounds::new(Coord { x: -180., y: 90. }, Coord { x: 180., y: -90. }),
                        2_usize.pow(level),
                        2_usize.pow(level),
                    )
                })
                .collect();

            BufferAllocator::new(levels, BUFFER_SIZE as usize, 0)
        };

        // let population_buffer_allocator = {
        //     let levels = (0..=6)
        //         .map(|level| {
        //             Level::new(
        //                 // Bounds::new(Coord { x: -180., y: 90. }, Coord { x: 180., y: -90. }),
        //                 Bounds::new(
        //                     Coord { x: -180., y: -72. },
        //                     Coord {
        //                         x: 179.99874,
        //                         y: 83.99958,
        //                     },
        //                 ),
        //                 2_usize.pow(level),
        //                 2_usize.pow(level),
        //             )
        //         })
        //         .collect();

        //     BufferAllocator::new(levels, BUFFER_SIZE as usize / 3, BUFFER_SIZE as usize / 3)
        // };
        let lp_buffer_allocator = {
            let levels = (0..=9)
                .map(|level| {
                    Level::new(
                        Bounds::new(Coord { x: -180., y: 90. }, Coord { x: 180., y: -90. }),
                        2_usize.pow(level),
                        2_usize.pow(level),
                    )
                })
                .collect();

            BufferAllocator::new(levels, BUFFER_SIZE as usize, 0)
        };

        Self {
            tile_map: HashMap::new(),
            buffer_allocator,

            lp_tile_map: HashMap::new(),
            lp_buffer_allocator,
            // population_tile_map: HashMap::new(),
            // population_buffer_allocator,
            eventloop,
            vertex_buffer,
            index_buffer,
            previous_output_as_lines: false,
            current_output_as_lines: false,
            update_tile_buffer: true,
            texture_bind_group_layout,

            icosphere,
            previous_subdivision_level: 1,
            current_subdivision_level: 0,
            num_vertices: 0,
            num_indices: 0,
            texture_buffer,
            texture_bind_group,
            texture_buffer_2,
            tile_metadata_buffer,
            tile_metadata_buffer_2,
            render_lp_map: false,
            last_buffer_write: web_time::Instant::now(),
            shader_mode_uniform,
            shader_mode: 0,
            // query_poi: QueryPoi::new(&device),
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

    pub fn set_render_lp_map(&mut self, render_lp_map: bool, queue: &Queue) {
        self.render_lp_map = render_lp_map;
        self.lp_buffer_allocator.reset();
        self.lp_tile_map = HashMap::new();
        self.update_tile_buffer = true;
        if self.render_lp_map {
            self.shader_mode = 1;
        } else {
            self.shader_mode = 0;
        }

        queue.write_buffer(
            &self.shader_mode_uniform,
            0,
            bytemuck::bytes_of(&[
                self.shader_mode,
                self.shader_mode,
                self.shader_mode,
                self.shader_mode,
            ]),
        );
    }

    pub fn set_subdivision_level(&mut self, level: usize) {
        self.current_subdivision_level = level;
    }

    pub fn set_output_to_lines(&mut self, output_as_lines: bool) {
        self.current_output_as_lines = output_as_lines;
    }
    pub fn test_bounding_box(&mut self, polygons: &[Coord<f32>], queue: &Queue) {
        for (i, polygon) in polygons.iter().enumerate() {
            let texture = vec![vec![[255, 0, 0, 255]; 256]; 256];

            let tile = TileResponse {
                data: texture,

                bounds: Rect::new(
                    coord! {x: polygon.x,y:polygon.y},
                    coord! {x: polygon.x+0.4,y:polygon.y+0.4},
                ),
            };

            let data = tile
                .get_padded_tile(TEXTURE_WIDTH, TEXTURE_HEIGHT)
                .into_iter()
                .flatten()
                .flatten()
                .collect::<Vec<u8>>();

            let metadata = TileMetadata::from((&tile, 0, 0));
            self.write_a_single_tile_to_buffer(&data, metadata, BufferSlot(i), queue);
            // #[cfg(feature = "debug")]
            // log::warn!("tile bounds! {:#?}", tile.bounds,);
        }

        self.update_tile_buffer = true;

        self.tile_map = HashMap::new();
    }
    pub fn update_visible_tiles(
        &mut self,
        projection: &Projection,
        camera: &Camera,
        _queue: &Queue,
    ) {
        let fov_intersections =
            calculate_camera_earth_view_bounding_box(projection, camera, Vec3::ZERO);

        // return self.test_bounding_box(&fov_intersections, _queue);

        let new_allocations = self.buffer_allocator.allocate(
            self.buffer_allocator.current_level as u32,
            &fov_intersections,
        );

        // let new_population_allocations = self.population_buffer_allocator.allocate(
        //     self.population_buffer_allocator.current_level as u32,
        //     &fov_intersections,
        // );

        let new_lp_allocations = self.lp_buffer_allocator.allocate(
            self.lp_buffer_allocator.current_level as u32,
            &fov_intersections,
        );

        let should_fetch_lp_tiles = self.render_lp_map;

        let proxy = self.eventloop.clone();
        wasm_bindgen_futures::spawn_local(async move {
            for tile_id in new_allocations {
                let tile: TileResponse<[u8; 4]> = bincode::deserialize(
                    &gloo_net::http::Request::get(&format!(
                        "/sat_tile/{}/{}/{}",
                        tile_id.0, tile_id.1, tile_id.2
                    ))
                    // .cache(web_sys::RequestCache::ForceCache)
                    .send()
                    .await
                    .unwrap()
                    .binary()
                    .await
                    .unwrap(),
                )
                .unwrap();

                proxy
                    .send_event(CustomEvent::HttpResponse(
                        crate::app::CustomResponseType::SatelliteImage(tile, tile_id),
                    ))
                    .unwrap();
            }

            // for tile_id in new_population_allocations {
            //     let tile: TileResponse<f32> = bincode::deserialize(
            //         &gloo_net::http::Request::get(&format!(
            //             "/pop_tile/{}/{}/{}",
            //             tile_id.0, tile_id.1, tile_id.2
            //         ))
            //         .cache(web_sys::RequestCache::ForceCache)
            //         .send()
            //         .await
            //         .unwrap()
            //         .binary()
            //         .await
            //         .unwrap(),
            //     )
            //     .unwrap();

            //     proxy
            //         .send_event(CustomEvent::HttpResponse(
            //             crate::app::CustomResponseType::PopulationTileResponse(tile, tile_id),
            //         ))
            //         .unwrap();
            // }
            //
            if !should_fetch_lp_tiles {
                return;
            }

            for tile_id in new_lp_allocations {
                let tile: TileResponse<f32> = bincode::deserialize(
                    &gloo_net::http::Request::get(&format!(
                        "/light_p_tile/{}/{}/{}",
                        tile_id.0, tile_id.1, tile_id.2
                    ))
                    .cache(web_sys::RequestCache::ForceCache)
                    .send()
                    .await
                    .unwrap()
                    .binary()
                    .await
                    .unwrap(),
                )
                .unwrap();

                proxy
                    .send_event(CustomEvent::HttpResponse(
                        crate::app::CustomResponseType::LightPollution(tile, tile_id),
                    ))
                    .unwrap();
            }
        });
    }

    pub fn update(&mut self, queue: &Queue, device: &Device) {
        if self.update_tile_buffer {
            self.rewrite_tiles(queue);
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

fn icosahedron_to_wgs84(v: Point) -> Point {
    let v = v.normalize();
    const EARTH_RADIUS: f32 = 1.;
    const FLATTENING: f32 = 1. / 298.257;

    let lat = convert_point_on_surface_to_lat_lon(v).y.to_radians();

    let flattening_factor = EARTH_RADIUS * (1.0 - FLATTENING);
    let lat_strech_axis_1 = EARTH_RADIUS * lat.sin();
    let lat_stech_axis_2 = flattening_factor * lat.cos();
    let r = EARTH_RADIUS * flattening_factor
        / (lat_strech_axis_1.powi(2) + lat_stech_axis_2.powi(2)).sqrt();
    v * r
}

fn ray_intersects_sphere(
    ray_origin: Vec3,
    ray_direction: Vec3,
    sphere_center: Vec3,
    sphere_radius: f32,
) -> Option<Vec3> {
    let origin_to_center = ray_origin - sphere_center;

    let a = ray_direction.dot(ray_direction);
    let b = 2.0 * ray_direction.dot(origin_to_center);
    let c = origin_to_center.dot(origin_to_center) - sphere_radius * sphere_radius;

    let discriminant = b * b - 4.0 * a * c;
    if discriminant < 0.0 {
        return None;
    }

    let sqrt_discriminant = discriminant.sqrt();
    let t0 = (-b - sqrt_discriminant) / (2.0 * a);
    let t1 = (-b + sqrt_discriminant) / (2.0 * a);

    if t0 < 0. && t1 < 0. {
        return None;
    }

    let closest_positive_t = if t0 >= 0.0 { t0 } else { t1 };

    Some(ray_origin + closest_positive_t * ray_direction)
}

fn convert_point_on_surface_to_lat_lon(point: Point) -> Coord<f32> {
    let point = point.normalize();
    let lon = if point.x == 0.0 && point.y == 0.0 {
        0.0
    } else {
        (-point.x.atan2(point.y)).to_degrees()
    };
    let lat = (-point.z).clamp(-1., 1.).asin().to_degrees();

    coord! {x:lon,y:lat}
}

fn calculate_camera_earth_view_bounding_box(
    camera_projection: &Projection,
    camera: &Camera,
    earth_position: Point,
) -> Vec<Coord<f32>> {
    const N_RAYS: usize = 6;
    let inv_view_proj = (camera_projection.calc_matrix() * camera.calc_matrix()).inverse();
    let cam_pos = inv_view_proj.project_point3(Vec3::ZERO);
    let cam_dir = -cam_pos.normalize();
    let (orth1, orth2) = cam_dir.any_orthonormal_pair();

    let fov = camera_projection.fovy;
    let half_fov = fov / 2.0;

    let mut surface_points = Vec::with_capacity(N_RAYS * N_RAYS);
    let angle_step = fov / (N_RAYS - 1) as f32;

    let mut angle_offsets = [0.0f32; N_RAYS];
    for (i, entry) in angle_offsets.iter_mut().enumerate().take(N_RAYS) {
        let mut angle_offset = -half_fov + i as f32 * angle_step;
        std::mem::swap(entry, &mut angle_offset);
    }

    for &angle_v in &angle_offsets {
        let qv = Quat::from_axis_angle(orth2, angle_v);
        for &angle_u in &angle_offsets {
            let qu = Quat::from_axis_angle(orth1, angle_u);
            let ray_dir = (qu * qv * cam_dir).normalize();
            if let Some(point) = ray_intersects_sphere(cam_pos, ray_dir, earth_position, 1.0) {
                surface_points.push(convert_point_on_surface_to_lat_lon(point));
            }
        }
    }

    surface_points
}
