use geo::{CoordsIter, LineString, Polygon};
use glam::{Mat3, Vec3, Vec3Swizzles, Vec4Swizzles};
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroupEntry, Buffer, BufferAddress, BufferDescriptor, BufferUsages, Device, Origin3d, Queue,
    RenderPass, SamplerDescriptor, ShaderStages, TexelCopyBufferLayout, TexelCopyTextureInfo,
    TextureAspect, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
    TextureViewDescriptor, VertexAttribute, VertexBufferLayout, VertexFormat, VertexStepMode,
};
pub type Point = glam::Vec3;

use crate::camera::{Camera, Projection};

use super::Icosphere;

#[derive(Debug)]
pub struct EarthState {
    vertex_buffer: Buffer,
    index_buffer: Buffer,

    icosphere: Icosphere,

    previous_subdivision_level: usize,
    current_subdivision_level: usize,
    previous_output_as_lines: bool,
    current_output_as_lines: bool,

    num_vertices: u32,
    num_indices: u32,
    texture_buffer: wgpu::Texture,
    texture_size: wgpu::Extent3d,
    current_texture: image::ImageBuffer<image::Rgba<u8>, Vec<u8>>,
    texture_bind_group: wgpu::BindGroup,
    pub texture_bind_group_layout: wgpu::BindGroupLayout,
}

impl EarthState {
    pub fn create(device: &Device) -> Self {
        let icosphere = Icosphere::new(1., Point::ZERO, 6, 0, vert_transform);

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

        // let texture_bytes = include_bytes!("../../checkerboard_test.png");
        let texture_bytes = include_bytes!("../../earthmap2k.jpg");
        let texture_img = image::load_from_memory(texture_bytes).unwrap();
        let texture_rgba = texture_img.to_rgba8();
        let texture_size = wgpu::Extent3d {
            width: texture_img.width(),
            height: texture_img.height(),
            depth_or_array_layers: 1,
        };
        let diffuse_texture = device.create_texture(&TextureDescriptor {
            label: Some("earth_texture_buffer"),
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8UnormSrgb,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let diffuse_texture_view = diffuse_texture.create_view(&TextureViewDescriptor::default());
        let diffuse_sampler = device.create_sampler(&SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
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
                            view_dimension: wgpu::TextureViewDimension::D2,
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
                ],
            });

        let diffuse_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
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
            ],
        });

        Self {
            vertex_buffer,
            index_buffer,
            texture_buffer: diffuse_texture,
            texture_size,
            previous_output_as_lines: false,
            current_output_as_lines: false,
            current_texture: texture_rgba,
            texture_bind_group: diffuse_bind_group,
            texture_bind_group_layout,

            icosphere,
            previous_subdivision_level: 1,
            current_subdivision_level: 0,
            num_vertices: 0,
            num_indices: 0,
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

    pub fn _t(&mut self, projection: &Projection, camera: &Camera) {
        let polygons = calculate_camera_earth_view_bounding_box(projection, camera, Point::ZERO);

        let texture = vec![vec![[255, 0, 0, 255]; 256]; 256];

        // let tile =
    }

    pub fn update(&mut self, queue: &Queue, device: &Device) {
        if self.current_subdivision_level == self.previous_subdivision_level
            && self.previous_output_as_lines == self.current_output_as_lines
        {
            return;
        }

        let (icosphere_verts, icosphere_faces) = if self.current_output_as_lines {
            self.icosphere
                .get_subdivison_level_vertecies_and_lines(self.current_subdivision_level)
        } else {
            queue.write_texture(
                TexelCopyTextureInfo {
                    texture: &self.texture_buffer,
                    mip_level: 0,
                    origin: Origin3d::ZERO,
                    aspect: TextureAspect::All,
                },
                &self.current_texture,
                TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(self.texture_size.width * 4),
                    rows_per_image: Some(self.texture_size.height),
                },
                self.texture_size,
            );

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
fn ray_sphere_intersect(
    origin_sphere: Point,
    line_point: Point,
    line_dir_vec: Point, // should be normalized
    radius: f32,
) -> Option<Point> {
    let oc = origin_sphere - line_point;
    let a = 1.0; // dir.dot(&dir) == 1 if normalized
    let b = 2.0 * oc.dot(line_dir_vec);
    let c = oc.dot(oc) - radius * radius;
    let discriminant = b * b - 4.0 * a * c;

    if discriminant < 0.0 {
        return None; // No intersection, make it so that it calculates the closest intersection along the line which is orthogonal to input line which crosses the origin
    }

    let sqrt_disc = discriminant.sqrt();
    let t = (-b + sqrt_disc.abs()) / (2.0 * a);

    if t <= 0.0 {
        return None;
    };

    Some(origin_sphere + line_dir_vec * t)
}

fn calculate_camera_earth_view_bounding_box(
    camera_projection: &Projection,
    camera: &Camera,
    earth_position: Point,
) -> Vec<geo::Polygon<f32>> {
    const MAX_BOUNDS: ((f32, f32), (f32, f32)) = ((90., -180.), (-90., -180.));
    let fov = camera_projection.fovy;
    let camera_pos = camera.orientation.xyz();

    // Given camera direction, create two vectors which represent the corners of the visible area
    // use the camera view vector and rotate it by fov angle in positive and negative using an orthogonal vector as the axis of rotation
    //
    //

    let camera_direction_vector = camera.calc_matrix().col(3).xyz();
    let (cam_orth_vector_1, cam_orth_vector_2) = camera_direction_vector.any_orthonormal_pair();

    let rotation_matrixes = [
        // Top-left: -fovx/2, +fovy/2
        (
            Mat3::from_axis_angle(cam_orth_vector_1, -fov / 2.0),
            Mat3::from_axis_angle(cam_orth_vector_2, fov / 2.0),
        ),
        // Top-right: +fovx/2, +fovy/2
        (
            Mat3::from_axis_angle(cam_orth_vector_1, fov / 2.0),
            Mat3::from_axis_angle(cam_orth_vector_2, fov / 2.0),
        ),
        // Bottom-left: -fovx/2, -fovy/2
        (
            Mat3::from_axis_angle(cam_orth_vector_1, -fov / 2.0),
            Mat3::from_axis_angle(cam_orth_vector_2, -fov / 2.0),
        ),
        // Bottom-right: +fovx/2, -fovy/2
        (
            Mat3::from_axis_angle(cam_orth_vector_1, fov / 2.0),
            Mat3::from_axis_angle(cam_orth_vector_2, -fov / 2.0),
        ),
    ];

    let mut fov_rays = [Vec3::ZERO; 4];
    for (i, (rm_1, rm_2)) in rotation_matrixes.iter().enumerate() {
        fov_rays[i] = *rm_2 * (*rm_1 * camera_direction_vector);
    }

    const WORLD_SPACE_EARTH_RADIUS: f32 = 1.; // Does not need to be accurate

    let surface_intersection_points = fov_rays
        .iter()
        .filter_map(|ray| {
            Some(convert_point_on_surface_to_lat_lon(ray_sphere_intersect(
                earth_position,
                camera_pos,
                *ray,
                WORLD_SPACE_EARTH_RADIUS,
            )?))
        })
        .collect::<Vec<(f32, f32)>>();

    let north_pole = Vec3::new(0., 0., 1.);
    let south_pole = Vec3::new(0., 0., -1.);

    let ray_to_north_pole = camera_pos - north_pole;
    let ray_to_south_pole = camera_pos - south_pole;

    let north_pole_is_visible = is_ray_in_cone(ray_to_north_pole, camera_direction_vector, fov);
    let south_pole_is_visible = is_ray_in_cone(ray_to_south_pole, camera_direction_vector, fov);

    if surface_intersection_points.is_empty() || south_pole_is_visible && north_pole_is_visible {
        // return MAX_BOUNDS;
        // return vec![Polygon::new(vec![MA], vec![])]
        todo!("Return max bounds polygon")
    }

    let mut out = Vec::new();

    // Crossing the meridian
    if surface_intersection_points[0].1 > surface_intersection_points[1].1 {
        out.push(Polygon::new(
            LineString::from(vec![
                surface_intersection_points[0],
                (surface_intersection_points[0].0, 180.),
                (surface_intersection_points[3].0, 180.),
                surface_intersection_points[3],
            ]),
            vec![],
        ));

        out.push(Polygon::new(
            LineString::from(vec![
                surface_intersection_points[1],
                (surface_intersection_points[1].0, -180.),
                (surface_intersection_points[2].0, -180.),
                surface_intersection_points[2],
            ]),
            vec![],
        ));
    }

    if north_pole_is_visible {
        for polygon in &mut out {
            for p in polygon.clone().coords_iter() {
                polygon.exterior_mut(|l| l.0.push((p.x, 90.).into()));
            }
        }
    }

    if south_pole_is_visible {
        for polygon in &mut out {
            for p in polygon.clone().coords_iter() {
                polygon.exterior_mut(|l| l.0.push((p.x, -90.).into()));
            }
        }
    }

    out
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

fn convert_point_on_surface_to_lat_lon(point: Point) -> (f32, f32) {
    let lon = point.x.atan2(-point.y).to_degrees();
    let lat = point.z.asin().to_degrees();

    (lat - 90., lon - 180.)
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
