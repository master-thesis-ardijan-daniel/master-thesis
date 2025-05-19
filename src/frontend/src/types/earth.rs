use std::collections::{HashMap, HashSet};

use common::{Bounds, TileMetadata, TileResponse};
use geo::{coord, Area, BoundingRect, Contains, Coord, Intersects, Rect};
use geo::{CoordsIter, LineString, Polygon};
use glam::{Mat3, Quat, Vec3, Vec3Swizzles, Vec4, Vec4Swizzles};
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
use crate::camera::{Camera, Projection};

use super::Icosphere;

type Point = Vec3;

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
    pub tiles: Vec<TileResponse<[u8; 4]>>,
    pub texture_bind_group_layout: wgpu::BindGroupLayout,
    tile_metadata_buffer: Buffer,
    eventloop: EventLoopProxy<CustomEvent>,
    finished_creation: bool,

    tiles_: Tiles,
    tile: HashMap<(u32, u32, u32), TileResponse<[u8; 4]>>,
}

impl EarthState {
    pub fn insert_tile(&mut self, id: (u32, u32, u32), data: TileResponse<[u8; 4]>) {
        #[cfg(feature = "debug")]
        log::warn!("Inserted tile {:#?}", id);

        self.tile.insert(id, data);
    }

    pub fn rewrite_tiles(&mut self, queue: &Queue) {
        let tiles = std::mem::take(&mut self.tile);

        for (id, tile) in tiles.into_iter() {
            let Some(&slot) = self.tiles_.allocated.get(&id) else {
                continue;
            };

            self.write_a_single_tile_to_buffer(tile, &slot, queue);
        }
    }

    pub fn write_a_single_tile_to_buffer(
        &mut self,
        new_tile: TileResponse<[u8; 4]>,
        slot: &BufferSlot,
        queue: &Queue,
    ) {
        queue.write_texture(
            TexelCopyTextureInfo {
                texture: &self.texture_buffer,
                mip_level: 0,
                origin: Origin3d {
                    x: 0,
                    y: 0,
                    z: slot.start as u32,
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
                depth_or_array_layers: 1,
            },
        );

        let metadata = TileMetadata::from(&new_tile);
        #[cfg(feature = "debug")]
        {
            log::warn!(
                "Metadata written: {:#?} at {}",
                metadata,
                (slot.start * size_of::<TileMetadata>()) as u64
            );
        }

        queue.write_buffer(
            &self.tile_metadata_buffer,
            (slot.start * size_of::<TileMetadata>()) as u64,
            bytemuck::bytes_of(&metadata),
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

                // #[cfg(feature = "debug")]
                // log::warn!("Tiles requested");

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
            size: size_of::<TileMetadata>() as u64 * 64,
            mapped_at_creation: false,
        });

        let texture_size = wgpu::Extent3d {
            width: TEXTURE_WIDTH,
            height: TEXTURE_HEIGHT,
            depth_or_array_layers: 64,
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

        let tiles = vec![TileResponse {
            data: vec![vec![[125u8; 4]; 256]; 256],
            bounds: Rect::new(coord! { x: -180., y:90.}, coord! { x: 180., y:-90.}),
        }];

        let levels = (0..3)
            .map(|level| {
                Level::new(
                    Bounds::new(Coord { x: -180., y: 90. }, Coord { x: 180., y: -90. }),
                    4_usize.pow(level),
                    4_usize.pow(level),
                )
            })
            .collect();

        Self {
            tiles_: Tiles::new(levels, 64),
            tile: HashMap::new(),

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

    pub fn tiling_logic(&mut self, projection: &Projection, camera: &Camera) {
        self.update_tile_buffer = true;
        let polygons = calculate_camera_earth_view_bounding_box(projection, camera, Point::ZERO);

        let fetch = self.tiles_.get_intersection(2, &polygons);

        for f in fetch {
            #[cfg(feature = "debug")]
            {
                log::warn!("fetching {:#?}", f);
            }
            let proxy = self.eventloop.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let tile = gloo_net::http::Request::get(&format!("/tile/{}/{}/{}", f.0, f.1, f.2))
                    .send()
                    .await
                    .unwrap()
                    .json()
                    .await
                    .unwrap();

                proxy
                    .send_event(CustomEvent::HttpResponse(
                        crate::app::CustomResponseType::TileResponse(tile, f),
                    ))
                    .unwrap();
            });
        }
    }

    pub fn update(&mut self, queue: &Queue, device: &Device) {
        if !self.finished_creation {
            self.finished_creation = true;
            // the response handler will set self.update_tiles_buffer to true;
            // self.fetch_tiles("/tiles".to_string());
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
    const PI: f32 = std::f32::consts::PI;
    pub fn to_lat_lon_range(point: Point) -> (f32, f32, f32) {
        let lenxy = point.xy().length();
        let range = point.length();

        if lenxy < 1.0e-10 {
            if point.z > 0. {
                return (PI / 2., 0.0, range);
            }
            (-(PI / 2.), 0.0, range)
        } else {
            let lat = point.z.atan2(lenxy);
            let lon = point.y.atan2(point.x);
            (lat, lon, range)
        }
    }

    v /= v.length();
    // const EARTH_RADIUS: 6378.137;
    const EARTH_RADIUS: f32 = 1.;
    // const FLATTENING: f32 = 1. / 298.257;
    const FLATTENING: f32 = 0.;
    // // get polar from cartesian
    let (lat, _lon, _rng) = to_lat_lon_range(v);
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

pub fn ray_sphere_intersect(origin: Vec3, direction: Vec3, center: Vec3, radius: f32) -> Vec3 {
    let oc = origin - center;
    let a = direction.dot(direction);
    let b = 2.0 * oc.dot(direction);
    let c = oc.dot(oc) - radius * radius;
    let discriminant = b * b - 4.0 * a * c;

    if discriminant < 0.0 {
        return forced_intersection(origin, direction, center, radius);
        // return None;
    }

    let sqrt_disc = discriminant.sqrt();
    let t1 = (-b - sqrt_disc) / (2.0 * a);
    let t2 = (-b + sqrt_disc) / (2.0 * a);

    let t = if t1 >= 0.0 {
        t1
    } else if t2 >= 0.0 {
        t2
    } else {
        return forced_intersection(origin, direction, center, radius);
    };

    origin + direction * t
}

fn forced_intersection(origin: Vec3, direction: Vec3, center: Vec3, radius: f32) -> Vec3 {
    // Vector from origin to center
    let to_center = center - origin;

    // Project to_center onto the plane perpendicular to direction
    let projection = to_center - direction * to_center.dot(direction);

    let fallback_direction = projection.normalize();

    // Intersect this fallback ray from center in fallback direction
    center + fallback_direction * radius
}

fn calculate_camera_earth_view_bounding_box(
    camera_projection: &Projection,
    camera: &Camera,
    earth_position: Point,
) -> Vec<geo::Polygon<f32>> {
    let cam_pos = (camera_projection.calc_matrix() * camera.calc_matrix())
        .inverse()
        .project_point3(-Vec3::Z);

    // Compute camera forward direction (negative Z in view space, transformed to world space)
    let camera_direction_vector = (Vec3::ZERO - cam_pos).normalize();

    let mut hit_points = vec![];
    let hit_point = ray_sphere_intersect(cam_pos, camera_direction_vector, Vec3::ZERO, 1.);
    hit_points.push(convert_point_on_surface_to_lat_lon(hit_point));

    let fov = camera_projection.fovy / 2.;

    // Compute orthonormal basis for camera
    let (cam_orth_vector_1, cam_orth_vector_2) = camera_direction_vector.any_orthonormal_pair();

    let rotation_matrices = [
        //Order matters
        (
            Mat3::from_axis_angle(cam_orth_vector_1, -fov / 2.0),
            Mat3::from_axis_angle(cam_orth_vector_2, -fov / 2.0),
        ),
        (
            Mat3::from_axis_angle(cam_orth_vector_1, -fov / 2.0),
            Mat3::from_axis_angle(cam_orth_vector_2, fov / 2.0),
        ),
        (
            Mat3::from_axis_angle(cam_orth_vector_1, fov / 2.0),
            Mat3::from_axis_angle(cam_orth_vector_2, fov / 2.0),
        ),
        (
            Mat3::from_axis_angle(cam_orth_vector_1, fov / 2.0),
            Mat3::from_axis_angle(cam_orth_vector_2, -fov / 2.0),
        ),
    ];

    let mut fov_rays = [Vec3::ZERO; 4];
    for (i, (rm_1, rm_2)) in rotation_matrices.iter().enumerate() {
        fov_rays[i] = (*rm_2 * (*rm_1 * camera_direction_vector)).normalize();
    }

    // Compute intersection points with Earth's surface
    let surface_intersection_points = fov_rays
        .iter()
        .filter_map(|ray| {
            let intersection = ray_sphere_intersect(cam_pos, *ray, earth_position, 1.);
            Some(convert_point_on_surface_to_lat_lon(intersection))
        })
        .collect::<Vec<Coord<f32>>>();

    let north_pole = Vec3::new(0., 0., 1.);
    let south_pole = Vec3::new(0., 0., -1.);

    hit_points.push(convert_point_on_surface_to_lat_lon(north_pole));
    hit_points.push(convert_point_on_surface_to_lat_lon(south_pole));
    let cam_distance_to_shpere_center = cam_pos.distance(Vec3::ZERO);

    let north_pole_is_visible = if north_pole.distance(cam_pos) > cam_distance_to_shpere_center {
        false
    } else {
        let ray_to_north_pole = (cam_pos - north_pole).normalize();

        is_vector_in_cone(-ray_to_north_pole, camera_direction_vector, fov)
    };

    let south_pole_is_visible = if south_pole.distance(cam_pos) > cam_distance_to_shpere_center {
        false
    } else {
        let ray_to_south_pole = (cam_pos - south_pole).normalize();

        is_vector_in_cone(-ray_to_south_pole, camera_direction_vector, fov / 2.)
    };

    let crossing_meridian = surface_intersection_points
        .clone()
        .iter()
        .fold(0.0, |acc, a| {
            surface_intersection_points
                .iter()
                .map(|b| {
                    let diff = (a.x - b.x).abs();
                    diff.max(diff)
                })
                .fold(acc, f32::max)
        })
        > 180.;
    let mut out = vec![];
    if crossing_meridian {
        out.push(vec![
            surface_intersection_points[0],
            coord! {x: 180.,y:surface_intersection_points[0].y},
            coord! {x: 180.,y:surface_intersection_points[3].y},
            surface_intersection_points[3],
        ]);

        out.push(vec![
            surface_intersection_points[1],
            coord! {x: -180.,y:surface_intersection_points[1].y},
            coord! {x: -180.,y:surface_intersection_points[2].y},
            surface_intersection_points[2],
        ]);
    } else {
        hit_points.extend(surface_intersection_points);
        out.push(hit_points);
    };

    if north_pole_is_visible {
        for poly_i in 0..out.len() {
            for point_i in 0..out[poly_i].len() {
                let c = out[poly_i][point_i];
                out[poly_i].push(coord! {x:c.x,y:-90.})
            }
        }
    }

    if south_pole_is_visible {
        for poly_i in 0..out.len() {
            for point_i in 0..out[poly_i].len() {
                let c = out[poly_i][point_i];
                out[poly_i].push(coord! {x:c.x,y:90.})
            }
        }
    }

    return out
        .into_iter()
        .map(|x| Polygon::new(LineString::from(x), vec![]))
        .collect();
}

fn convert_point_on_surface_to_lat_lon(point: Point) -> Coord<f32> {
    // #[cfg(feature = "debug")]
    // log::warn!("DDD1 {:#?}", point);
    let lon = if point.x == 0.0 && point.y == 0.0 {
        0.0
    } else {
        point.x.atan2(-point.y).to_degrees()
    };
    let lat = -point.z.clamp(-1., 1.).asin().to_degrees();

    coord! {x:lon,y:lat}
}

fn is_vector_in_cone(vector: Vec3, cone_axis: Vec3, cone_angle: f32) -> bool {
    let cos_angle = vector.dot(cone_axis).clamp(-1., 1.);

    cos_angle >= cone_angle.cos()
}

#[derive(Debug)]
struct Tiles {
    levels: Vec<Level>,
    visible: HashSet<(u32, u32, u32)>,

    allocated: HashMap<(u32, u32, u32), BufferSlot>,
    free: Vec<BufferSlot>,
}

#[derive(Debug, Copy, Clone)]
struct BufferSlot {
    start: usize,
}

#[derive(Debug)]
struct Level {
    bounds: Bounds,
    width: usize,
    height: usize,
    step_x: f32,
    step_y: f32,
}

impl Level {
    pub fn new(bounds: Bounds, width: usize, height: usize) -> Self {
        let step_x = bounds.height() / height as f32;
        let step_y = bounds.width() / width as f32;

        Self {
            bounds,
            width,
            height,
            step_x,
            step_y,
        }
    }
}

impl Tiles {
    pub fn new(levels: Vec<Level>, slots: usize) -> Self {
        let free = (0..slots)
            .rev()
            .map(|start| BufferSlot { start })
            .collect::<Vec<_>>();

        #[cfg(feature = "debug")]
        log::warn!("Allocated {} slots", free.len());

        Self {
            levels,

            visible: HashSet::new(),
            free,
            allocated: HashMap::new(),
        }
    }

    pub fn get_intersection(&mut self, z: u32, polygons: &[Polygon<f32>]) -> Vec<(u32, u32, u32)> {
        let level = &self.levels[z as usize];

        let mut visible = HashSet::new();

        for polygon in polygons {
            let bounds = polygon.bounding_rect().unwrap();

            let min_x = (bounds.min().x / level.step_x) as u32;
            let min_y = (bounds.min().y / level.step_y) as u32;
            let max_x = (bounds.max().x / level.step_x) as u32;
            let max_y = (bounds.max().y / level.step_y) as u32;

            for y in min_y..max_y {
                for x in min_x..max_x {
                    visible.insert((z, y, x));
                }
            }
        }

        let not_visible_anymore = self.visible.difference(&visible);

        for tile in not_visible_anymore {
            if let Some(deallocated) = self.allocated.remove(tile) {
                self.free.push(deallocated);
            }
        }

        let to_be_allocated = visible
            .difference(&self.visible)
            .copied()
            .collect::<Vec<_>>();

        for &tile in &to_be_allocated {
            if let Some(slot) = self.free.pop() {
                self.allocated.insert(tile, slot);
            }
        }

        self.visible = visible;

        to_be_allocated
    }
}
