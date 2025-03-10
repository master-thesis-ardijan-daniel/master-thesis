use crate::{
    camera::CameraState,
    safe_get_subdivision_level,
    types::{Icosphere, Point},
};
use web_time::Duration;
use wgpu::{util::DeviceExt, FragmentState};
use winit::window::Window;

mod input;

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

    camera_state: CameraState,

    pub delta: Duration,
    earth: Icosphere,
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

        fn vert_transform(mut v: Point) -> Point {
            let vec_length = (v.x().powi(2) + v.y().powi(2) + v.z().powi(2)).sqrt();
            v /= vec_length;
            // const EARTH_RADIUS: 6378.137;
            const EARTH_RADIUS: f32 = 1.;
            // const FLATTENING: f32 = 1. / 298.257;
            const FLATTENING: f32 = 0.3;
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

        let mut icosphere = Icosphere::new(1., Point::new_zero(), 6, 0, vert_transform);

        let (icosphere_verts, icosphere_lines) =
            icosphere.get_subdivison_level_vertecies_and_lines(0);

        let mut icosahedorn_vertecies = vec![];
        for vc in icosphere_verts {
            icosahedorn_vertecies.push(Vertex {
                position: vc.to_array(),
                color: [0.5, 0., 0.5],
            });
        }

        let mut lines = icosphere_lines
            .clone()
            .as_flattened()
            .iter()
            .map(|x| u16::try_from(*x).unwrap())
            .collect::<Vec<u16>>();
        lines.push(0);

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

        let camera_state = CameraState::create(&device, &size);

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&camera_state.bind_group_layout],
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
            num_vertices: icosphere_verts.len() as u32,
            index_buffer,
            num_indices: lines.len() as u32,
            earth: icosphere,
            camera_state,
            delta: Duration::ZERO,
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
            self.camera_state
                .controller
                .resize(new_size.width, new_size.height);
        }
    }

    pub fn update(&mut self) {
        self.camera_state.update(&self.queue, self.delta);
    }

    pub fn check_earth_subid_level(&mut self) {
        let frontend_subdiv_level = if let Some(v) = safe_get_subdivision_level() {
            if v == self.earth.current_subdiv_level {
                return;
            }
            v
        } else {
            return;
        };

        let (icosphere_verts, icosphere_lines) = self
            .earth
            .get_subdivison_level_vertecies_and_lines(frontend_subdiv_level);

        let mut icosahedorn_vertecies = vec![];
        for vc in icosphere_verts {
            icosahedorn_vertecies.push(Vertex {
                position: vc.to_array(),
                color: [0.5, 0., 0.5],
            });
        }

        let mut lines = icosphere_lines
            .clone()
            .as_flattened()
            .iter()
            .map(|x| u16::try_from(*x).unwrap())
            .collect::<Vec<u16>>();
        lines.push(0);

        self.num_vertices = icosphere_verts.len() as u32;
        self.earth.current_subdiv_level = frontend_subdiv_level;
        self.num_indices = lines.len() as u32;
        self.vertex_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Vertex buffer"),
                contents: bytemuck::cast_slice(&icosahedorn_vertecies),
                usage: wgpu::BufferUsages::VERTEX,
            });

        self.index_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(&lines),
                usage: wgpu::BufferUsages::INDEX,
            });
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        self.check_earth_subid_level();
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
            render_pass.set_bind_group(0, &self.camera_state.bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}
