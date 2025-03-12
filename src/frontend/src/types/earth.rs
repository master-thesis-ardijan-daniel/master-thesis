use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    Buffer, BufferAddress, BufferDescriptor, BufferUsages, Device, Queue, RenderPass,
    VertexAttribute, VertexBufferLayout, VertexFormat, VertexStepMode,
};

use super::{Icosphere, Point};

pub struct EarthState {
    vertex_buffer: Buffer,
    index_buffer: Buffer,

    icosphere: Icosphere,

    previous_subdivision_level: usize,
    current_subdivision_level: usize,

    pub num_vertices: u32,
    pub num_indices: u32,
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

        Self {
            vertex_buffer,
            index_buffer,

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

    pub fn update(&mut self, _queue: &Queue, device: &Device) {
        if self.current_subdivision_level == self.previous_subdivision_level {
            return;
        }

        let (icosphere_verts, icosphere_lines) = self
            .icosphere
            .get_subdivison_level_vertecies_and_lines(self.current_subdivision_level);

        self.num_vertices = icosphere_verts.len() as u32;
        self.num_indices = icosphere_lines.len() as u32 * 2;

        let icosphere_verts = icosphere_verts
            .iter()
            .map(Point::to_array)
            .collect::<Vec<_>>();

        let mut icosphere_lines = icosphere_lines
            .as_flattened()
            .iter()
            .map(|x| u16::try_from(*x).unwrap())
            .collect::<Vec<u16>>();
        icosphere_lines.push(0);

        self.vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&icosphere_verts),
            usage: BufferUsages::VERTEX,
        });

        self.index_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&icosphere_lines),
            usage: BufferUsages::INDEX,
        });

        self.previous_subdivision_level = self.current_subdivision_level;
    }

    pub fn render(&self, render_pass: &mut RenderPass<'_>) -> u32 {
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);

        self.num_indices
    }
}

fn vert_transform(mut v: Point) -> Point {
    v /= v.length();
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
