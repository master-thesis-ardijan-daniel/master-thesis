use std::{f32::consts::PI, sync::Arc};

use crate::{
    camera::{Camera, CameraState, Projection},
    types::{earth::EarthState, Point},
};
use glam::{Mat3, Vec3, Vec4Swizzles};
use touch::TouchState;
use web_time::Duration;
use wgpu::FragmentState;
use winit::window::Window;

mod input;
mod touch;

pub enum AnimationState {
    Animating,
    Finished,
}

#[derive(Debug)]
pub struct State {
    pub surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub size: winit::dpi::PhysicalSize<u32>,
    pub window: Arc<Window>,
    pub texture_pipeline: wgpu::RenderPipeline,
    pub wireframe_pipeline: wgpu::RenderPipeline,
    render_wireframe: bool,

    touch_state: TouchState,

    pub camera_state: CameraState,
    pub earth_state: EarthState,

    pub delta: Duration,
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
    camera_projection: Projection,
    camera: Camera,
    earth_position: Point,
) -> ((f32, f32), (f32, f32)) {
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

    if surface_intersection_points.is_empty() {
        return MAX_BOUNDS;
    }

    let north_pole = Vec3::new(0., 0., 1.);
    let south_pole = Vec3::new(0., 0., -1.);

    let ray_to_north_pole = camera_pos - north_pole;
    let ray_to_south_pole = camera_pos - south_pole;

    let north_pole_is_visible = is_ray_in_cone(ray_to_north_pole, camera_direction_vector, fov);
    let south_pole_is_visible = is_ray_in_cone(ray_to_south_pole, camera_direction_vector, fov);

    let nw_lat = if north_pole_is_visible {
        90.
    } else {
        surface_intersection_points
            .iter()
            .fold(surface_intersection_points[0].0, |a, &b| a.max(b.0))
    };

    let se_lat = if south_pole_is_visible {
        -90.
    } else {
        surface_intersection_points
            .iter()
            .fold(surface_intersection_points[0].0, |a, &b| a.min(b.0))
    };

    // Find longest distance to other points
    let max_diff = surface_intersection_points
        .iter()
        .fold(0.0, |acc, &(_, lon)| {
            surface_intersection_points
                .iter()
                .map(|&(_, other)| {
                    let diff = (lon - other).abs();
                    diff.min(360. - diff)
                })
                .fold(acc, f32::max)
        });

    let (nw_lon, se_lon) = if max_diff > 180. {
        // Meridian crossing: find min/max considering wraparound
        let mut min_lon = surface_intersection_points[0].1;
        let mut max_lon = surface_intersection_points[0].1;
        for &(_, lon) in surface_intersection_points.iter().skip(1) {
            if (lon - min_lon + 360.) % 360. > 180. {
                min_lon = lon;
            }
            if (max_lon - lon + 360.) % 360. > 180. {
                max_lon = lon;
            }
        }
        (min_lon, max_lon)
    } else {
        let min_lon = surface_intersection_points
            .iter()
            .fold(surface_intersection_points[0].1, |a, &(_, b)| a.min(b));
        let max_lon = surface_intersection_points
            .iter()
            .fold(surface_intersection_points[0].1, |a, &(_, b)| a.max(b));
        (min_lon, max_lon)
    };

    ((nw_lat, nw_lon), (se_lat, se_lon))
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

impl State {
    pub async fn new(window: Arc<Window>) -> State {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::GL,
            ..Default::default()
        });

        let surface = instance
            .create_surface(window.clone())
            .expect("Unable to create surface");

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("Requesting adapter failed!");

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                        .using_resolution(adapter.limits()),
                    ..Default::default()
                },
                None,
            )
            .await
            .unwrap();

        {
            let config = surface
                .get_default_config(&adapter, size.width.max(1), size.height.max(1))
                .unwrap();

            surface.configure(&device, &config);
        }

        let shader = device.create_shader_module(wgpu::include_wgsl!("../shaders/shader.wgsl"));

        let surface_caps = surface.get_capabilities(&adapter);

        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        let camera_state = CameraState::create(&device, &size);
        let earth_state = EarthState::create(&device);

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[
                &camera_state.bind_group_layout,
                &earth_state.texture_bind_group_layout,
            ],
            push_constant_ranges: &[],
        });

        let texture_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[EarthState::descriptor()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent::REPLACE,
                        alpha: wgpu::BlendComponent::REPLACE,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        let wireframe_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[EarthState::descriptor()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: Some("fs_wireframe"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent::REPLACE,
                        alpha: wgpu::BlendComponent::REPLACE,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        Self {
            surface,
            device,
            queue,
            config,
            size,
            window,
            texture_pipeline,
            wireframe_pipeline,
            touch_state: Default::default(),
            earth_state,
            camera_state,
            delta: Duration::ZERO,
            render_wireframe: false,
        }
    }

    pub fn window(&self) -> &Window {
        self.window.as_ref()
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
            self.camera_state
                .controller
                .resize(new_size.width, new_size.height);
        }
    }

    pub fn update(&mut self) {
        self.earth_state.update(&self.queue, &self.device);

        if let AnimationState::Animating = self.camera_state.update(&self.queue, self.delta) {
            self.window.request_redraw();
        }
    }

    pub fn set_render_wireframe(&mut self, render_as_wireframe: bool) {
        self.render_wireframe = render_as_wireframe;
        self.earth_state.set_output_to_lines(render_as_wireframe);
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Main render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.01,
                            g: 0.01,
                            b: 0.01,
                            a: 1.,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            if self.render_wireframe {
                render_pass.set_pipeline(&self.wireframe_pipeline);
            } else {
                render_pass.set_pipeline(&self.texture_pipeline);
            }

            let mut indices = 0;

            indices += self.camera_state.render(&mut render_pass);
            indices += self.earth_state.render(&mut render_pass);

            render_pass.draw_indexed(0..indices, 0, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}
