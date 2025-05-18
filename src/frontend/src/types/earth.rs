use common::{Bounds, TileMetadata, TileResponse};
use geo::{coord, BoundingRect, Coord, Rect};
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

        #[cfg(feature = "debug")]
        log::warn!(" tile_metadata {:#?}", tile_metadata);

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
        new_tile: TileResponse<[u8; 4]>,
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

        let tiles = vec![TileResponse {
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

    pub fn tiling_logic(&mut self, projection: &Projection, camera: &Camera) {
        self.update_tile_buffer = true;
        let polygons = calculate_camera_earth_view_bounding_box(projection, camera, Point::ZERO);

        let mut out = Vec::new();
        for polygon in &polygons {
            for point in polygon.coords_iter() {
                let texture = vec![vec![[255, 0, 0, 255]; 256]; 256];

                let tile = TileResponse {
                    data: texture,
                    bounds: Bounds::new(
                        coord! {x: point.x +1., y: point.y+1.},
                        coord! {x: point.x -1., y: point.y-1.},
                    ),
                };

                #[cfg(feature = "debug")]
                log::warn!("tile bounds! {:#?}", tile.bounds,);
                out.push(tile);
            }
        }

        // #[cfg(feature = "debug")]
        // log::warn!("tiles {:#?}", self.tile_metadata,);

        self.tiles = out;
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

pub fn ray_sphere_intersect(
    origin: Vec3,
    direction: Vec3,
    center: Vec3,
    radius: f32,
) -> Option<Vec3> {
    let oc = origin - center;
    let a = direction.dot(direction);
    let b = 2.0 * oc.dot(direction);
    let c = oc.dot(oc) - radius * radius;
    let discriminant = b * b - 4.0 * a * c;

    if discriminant < 0.0 {
        return None;
    }

    let sqrt_disc = discriminant.sqrt();
    let t1 = (-b - sqrt_disc) / (2.0 * a);
    let t2 = (-b + sqrt_disc) / (2.0 * a);

    let t = if t1 >= 0.0 {
        t1
    } else if t2 >= 0.0 {
        t2
    } else {
        return None;
    };

    Some(origin + direction * t)
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
    if hit_point.is_none() {
        #[cfg(feature = "debug")]
        log::warn!("No intersection!");
        return vec![];
    } else {
        hit_points.push(convert_point_on_surface_to_lat_lon(hit_point.unwrap()));
    }

    #[cfg(feature = "debug")]
    log::warn!("Intersection: {:?}", hit_point);

    let fov = camera_projection.fovy;

    // Compute orthonormal basis for camera
    let (cam_orth_vector_1, cam_orth_vector_2) = camera_direction_vector.any_orthonormal_pair();

    let rotation_matrices = [
        (
            Mat3::from_axis_angle(cam_orth_vector_1, fov / 4.0),
            Mat3::from_axis_angle(cam_orth_vector_2, fov / 4.0),
        ),
        (
            Mat3::from_axis_angle(cam_orth_vector_1, fov / 4.0),
            Mat3::from_axis_angle(cam_orth_vector_2, -fov / 4.0),
        ),
        (
            Mat3::from_axis_angle(cam_orth_vector_1, -fov / 4.0),
            Mat3::from_axis_angle(cam_orth_vector_2, fov / 4.0),
        ),
        (
            Mat3::from_axis_angle(cam_orth_vector_1, -fov / 4.0),
            Mat3::from_axis_angle(cam_orth_vector_2, -fov / 4.0),
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
            let intersection = ray_sphere_intersect(cam_pos, *ray, earth_position, 1.)?;
            Some(convert_point_on_surface_to_lat_lon(intersection))
        })
        .collect::<Vec<Coord<f32>>>();

    hit_points.extend(surface_intersection_points);

    // let new_hit = ray_sphere_intersect(cam_pos, fov_rays[0], earth_position, 1.);

    // if new_hit.is_none() {
    //     #[cfg(feature = "debug")]
    //     log::warn!("NEW HIT!!!! No intersection!");
    //     return vec![Polygon::new(LineString::from(hit_points), vec![])];
    //     // return ;
    // } else {
    //     hit_points.push(convert_point_on_surface_to_lat_lon(new_hit.unwrap()));
    // }

    // :ray_sphere_intersect(cam_pos, *ray, earth_position, 1.)    hit_points.push();
    return vec![Polygon::new(LineString::from(hit_points), vec![])];

    // const WORLD_SPACE_EARTH_RADIUS: f32 = 1.; // Does not need to be accurate

    // let north_pole = Vec3::new(0., 0., 1.);
    // let south_pole = Vec3::new(0., 0., -1.);

    // let ray_to_north_pole = camera_pos - north_pole;
    // let ray_to_south_pole = camera_pos - south_pole;

    // let north_pole_is_visible = is_ray_in_cone(ray_to_north_pole, camera_direction_vector, fov);
    // let south_pole_is_visible = is_ray_in_cone(ray_to_south_pole, camera_direction_vector, fov);

    // if surface_intersection_points.is_empty() || south_pole_is_visible && north_pole_is_visible {
    //     // return MAX_BOUNDS;
    //     // todo!("Return max bounds polygon")
    //     #[cfg(feature = "debug")]
    //     log::warn!("Hit max bounds");
    //     // return vec![Polygon::new(vec![MA], vec![])]
    //     return vec![];
    // }

    // let mut out = Vec::new();

    // // Crossing the meridian
    // if surface_intersection_points[0].1 > surface_intersection_points[1].1 {
    //     out.push(Polygon::new(
    //         LineString::from(vec![
    //             surface_intersection_points[0],
    //             (surface_intersection_points[0].0, 180.),
    //             (surface_intersection_points[3].0, 180.),
    //             surface_intersection_points[3],
    //         ]),
    //         vec![],
    //     ));

    //     out.push(Polygon::new(
    //         LineString::from(vec![
    //             surface_intersection_points[1],
    //             (surface_intersection_points[1].0, -180.),
    //             (surface_intersection_points[2].0, -180.),
    //             surface_intersection_points[2],
    //         ]),
    //         vec![],
    //     ));
    // }

    // if north_pole_is_visible {
    //     for polygon in &mut out {
    //         for p in polygon.clone().coords_iter() {
    //             polygon.exterior_mut(|l| l.0.push((p.x, 90.).into()));
    //         }
    //     }
    // }

    // if south_pole_is_visible {
    //     for polygon in &mut out {
    //         for p in polygon.clone().coords_iter() {
    //             polygon.exterior_mut(|l| l.0.push((p.x, -90.).into()));
    //         }
    //     }
    // }

    // out
    // // Find longest distance to other points
    // let max_diff = surface_intersection_points
    //     .iter()
    //     .fold(0.0, |acc, &(_, lon)| {
    //         surface_intersection_points
    //             .iter()
    //             .map(|&(_, other)| {
    //                 let diff = (lon - other).abs();
    //                 diff.min(360. - diff)
    //             })
    //             .fold(acc, f32::max)
    //     });

    // let mut view_boxes = vec![];

    // let (nw_lon, se_lon) = if max_diff > 180. {
    //     // Meridian crossing: find min/max considering wraparound
    //     let mut min_lon = surface_intersection_points[0].1;
    //     let mut max_lon = surface_intersection_points[0].1;
    //     for &(_, lon) in surface_intersection_points.iter().skip(1) {
    //         if (lon - min_lon + 360.) % 360. > 180. {
    //             min_lon = lon;
    //         }
    //         if (max_lon - lon + 360.) % 360. > 180. {
    //             max_lon = lon;
    //         }
    //     }
    //     (min_lon, max_lon)
    // } else {
    //     let min_lon = surface_intersection_points
    //         .iter()
    //         .fold(surface_intersection_points[0].1, |a, &(_, b)| a.min(b));
    //     let max_lon = surface_intersection_points
    //         .iter()
    //         .fold(surface_intersection_points[0].1, |a, &(_, b)| a.max(b));
    //     (min_lon, max_lon)
    // };

    // ((nw_lat, nw_lon), (se_lat, se_lon))
}

fn convert_point_on_surface_to_lat_lon(point: Point) -> Coord<f32> {
    #[cfg(feature = "debug")]
    log::warn!("DDD1 {:#?}", point);
    let lon = if point.x == 0.0 && point.y == 0.0 {
        0.0
    } else {
        point.x.atan2(-point.y).to_degrees()
    };
    // let lon = point.x.atan2(-point.y).to_degrees();
    let lat = -point.z.clamp(-1., 1.).asin().to_degrees();

    coord! {x:lon,y:lat}
}

fn is_ray_in_cone(ray_dir: Point, cone_dir: Point, cone_half_angle_rad: f32) -> bool {
    let ray_dir = ray_dir.normalize();
    let cone_dir = cone_dir.normalize();
    let cos_half_angle = cone_half_angle_rad.cos();

    ray_dir.dot(cone_dir) >= cos_half_angle
}

// fn linear_lat_lon_to_indicies(lat: f32,lon:f32)->{

// }

// for each frame on update, use the visible area struct to check which tiles are visible and which are not.
// Tiles which are not visible can be marked and can be replaced.
//
//
// you know which level you are on, and how many tiles there should be on that level, thus you can calculate which tile you need.
//
//
//
//
//

// Given an area, defined by 2 coordinates we can find which tiles should be in the buffer
// then we can check with a hashmap or something to figure out if it actually is, and thus find the missing ones.

fn tile_fetch_logic(
    level: u32,
    n_tiles_lat: u32,
    n_tiles_lon: u32,
    north_west: (f32, f32),
    south_east: (f32, f32),
) {
    let lat_step = 180. / n_tiles_lat as f32;
    let lon_step = 360. / n_tiles_lat as f32;

    let north_west_x = north_west.1 / lat_step;
    let north_west_y = north_west.0 / lon_step;

    let tiles_which_should_be_visible = todo!();
}
