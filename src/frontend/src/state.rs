use crate::{
    camera,
    sphere::{make_rotated_icosahedron, subdivide_icosphere},
};
use cgmath::{Matrix4, SquareMatrix};
use web_time::Duration;
use wgpu::{util::DeviceExt, FragmentState};
use winit::{
    event::*,
    keyboard::PhysicalKey,
    window::{CursorIconParseError, Window},
};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
}

impl Vertex {
    fn descriptor() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
    view_position: [f32; 4],
    view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    fn new() -> Self {
        Self {
            view_position: [0.; 4],
            view_proj: Matrix4::identity().into(),
        }
    }

    fn update_view_proj(&mut self, camera: &camera::Camera, projection: &camera::Projection) {
        self.view_position = camera.position.to_homogeneous().into();

        self.view_proj = (projection.calc_matrix() * camera.calc_matrix()).into();
    }
}

pub struct State<'a> {
    pub surface: wgpu::Surface<'a>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub size: winit::dpi::PhysicalSize<u32>,
    pub window: &'a Window,
    pub pipeline: wgpu::RenderPipeline,
    pub vertex_buffer: wgpu::Buffer,
    pub num_vertices: u32,
    pub index_buffer: wgpu::Buffer,
    pub num_indices: u32,

    pub camera: camera::Camera,
    pub projection: camera::Projection,
    pub mouse_pressed: bool,
    pub camera_controller: camera::CameraController,
    camera_bind_group: wgpu::BindGroup,
    camera_uniform: CameraUniform,
    camera_buffer: wgpu::Buffer,
}

impl<'a> State<'a> {
    pub async fn new(window: &'a Window) -> State<'a> {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::GL,
            ..Default::default()
        });

        let surface = instance
            .create_surface(window)
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
                    required_limits: wgpu::Limits::downlevel_webgl2_defaults(),
                    ..Default::default()
                },
                None,
            )
            .await
            .unwrap();

        let dist_function = |v: &mut crate::types::Point| {
            let vec_length = (v.x().powi(2) + v.y().powi(2) + v.z().powi(2)).sqrt();
            *v /= vec_length;
        };

        let (icosahedron_vert_coords, icosahedron_faces) = make_rotated_icosahedron();
        let (icosahedron_vert_coords, icosahedron_faces) =
            subdivide_icosphere(&icosahedron_vert_coords, &icosahedron_faces, dist_function);
        let (icosahedron_vert_coords, icosahedron_faces) =
            subdivide_icosphere(&icosahedron_vert_coords, &icosahedron_faces, dist_function);
        let (icosahedron_vert_coords, icosahedron_faces) =
            subdivide_icosphere(&icosahedron_vert_coords, &icosahedron_faces, dist_function);
        let (icosahedron_vert_coords, icosahedron_faces) =
            subdivide_icosphere(&icosahedron_vert_coords, &icosahedron_faces, dist_function);
        let (icosahedron_vert_coords, icosahedron_faces) =
            subdivide_icosphere(&icosahedron_vert_coords, &icosahedron_faces, dist_function);
        let (icosahedron_vert_coords, icosahedron_faces) =
            subdivide_icosphere(&icosahedron_vert_coords, &icosahedron_faces, dist_function);
        let (icosahedron_vert_coords, icosahedron_faces) =
            subdivide_icosphere(&icosahedron_vert_coords, &icosahedron_faces, dist_function);
        let (icosahedron_vert_coords, icosahedron_faces) =
            subdivide_icosphere(&icosahedron_vert_coords, &icosahedron_faces, dist_function);
        let (icosahedron_vert_coords, icosahedron_faces) =
            subdivide_icosphere(&icosahedron_vert_coords, &icosahedron_faces, dist_function);

        let mut icosahedorn_vertecies = vec![];
        for vc in icosahedron_vert_coords.clone() {
            icosahedorn_vertecies.push(Vertex {
                position: vc.to_array(),
                color: [0.5, 0., 0.5],
            });
        }

        let mut indicies = icosahedron_faces
            .clone()
            .as_flattened()
            .iter()
            .map(|x| u16::try_from(*x).unwrap())
            .collect::<Vec<u16>>();
        indicies.push(0);

        let mut lines = vec![];
        for face in icosahedron_faces.clone() {
            lines.push(face[0] as u16);
            lines.push(face[1] as u16);
            lines.push(face[1] as u16);
            lines.push(face[2] as u16);
            lines.push(face[2] as u16);
            lines.push(face[0] as u16);
        }

        let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex buffer"),
            contents: bytemuck::cast_slice(&icosahedorn_vertecies),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(&lines),
            usage: wgpu::BufferUsages::INDEX,
        });

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

        let camera = camera::Camera::new((0., 1., 1.), cgmath::Deg(-90.), cgmath::Deg(-20.));

        let projection = camera::Projection::new(450, 450, cgmath::Deg(20.), 0.1, 100.);
        let camera_controller = camera::CameraController::new(4., 2.5);

        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_view_proj(&camera, &projection);

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("camera_bind_group_layout"),
            });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&camera_bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::descriptor()],
                // buffers: &[],
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
            pipeline,
            vertex_buffer,
            num_vertices: icosahedron_vert_coords.len() as u32,
            index_buffer,
            num_indices: icosahedron_faces.len() as u32 * 3 * 2,
            mouse_pressed: false,
            camera,
            projection,
            camera_controller,
            camera_bind_group,
            camera_uniform,
            camera_buffer,
        }
    }

    pub fn window(&self) -> &Window {
        self.window
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    pub fn input(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(key),
                        state,
                        ..
                    },
                ..
            } => self.camera_controller.process_keyboard(*key, *state),
            WindowEvent::MouseInput { state, .. } => {
                self.mouse_pressed = state.is_pressed();
                true
            }
            _ => false,
        }
    }

    pub fn update(&mut self, duration: Duration) {
        self.camera_controller
            .update_camera(&mut self.camera, duration);
        self.camera_uniform
            .update_view_proj(&self.camera, &self.projection);
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[self.camera_uniform]),
        );
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
                            r: 0.1,
                            g: 0.1,
                            b: 0.1,
                            a: 1.,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}
