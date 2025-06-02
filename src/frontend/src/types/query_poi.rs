use glam::{Vec2, Vec3};
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BufferAddress, BufferDescriptor, BufferUsages, Device, Queue, QueueWriteBufferView,
    VertexAttribute, VertexFormat, VertexStepMode,
};

#[derive(Debug)]
pub struct QueryPoi {
    querying: bool,
    start_pos: Option<Vec3>,
    radius: f32,
    query_poi_uniform: wgpu::Buffer,
    value_changed: bool,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ShaderQueryPoi {
    start_pos: [f32; 3],
    radius: f32,
}

impl TryInto<ShaderQueryPoi> for &QueryPoi {
    fn try_into(self) -> Result<ShaderQueryPoi, Self::Error> {
        Ok(ShaderQueryPoi {
            start_pos: self
                .start_pos
                .ok_or("No starting position")?
                .clone()
                .to_array(),
            radius: self.radius,
        })
    }

    type Error = &'static str;
}

impl QueryPoi {
    pub fn new(device: &Device) -> Self {
        let query_poi_uniform = device.create_buffer(&BufferDescriptor {
            label: Some("query_poi_data_buffer"),
            size: size_of::<ShaderQueryPoi>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        QueryPoi {
            query_poi_uniform,
            querying: false,
            start_pos: None,
            radius: 10.,
            value_changed: false,
        }
    }

    fn calculate_intersection(&mut self, mouse_dx: f64, mouse_dy: f64) -> Option<Vec3> {
        todo!()
        // self.start_pos
    }

    pub fn update(&self, queue: &Queue) {
        if self.value_changed {
            if let Ok(pos) = TryInto::<ShaderQueryPoi>::try_into(self) {
                queue.write_buffer(&self.query_poi_uniform, 0, bytemuck::bytes_of(&pos));
            }
        }
    }

    pub fn process_cursor_moved(&mut self, mouse_dx: f64, mouse_dy: f64) {
        if !self.querying {
            return;
        }
        if self.start_pos.is_none() {
            self.start_pos = self.calculate_intersection(mouse_dx, mouse_dy);

            return;
        }

        if let (Some(end_pos), Some(start_pos)) = (
            self.calculate_intersection(mouse_dx, mouse_dy),
            self.start_pos,
        ) {
            let new_radius = (end_pos - start_pos).length();

            self.value_changed = self.radius != new_radius;
        }
    }
    pub fn start_query_poi(&mut self) {
        self.querying = true;
    }
    pub fn end_query_poi(&mut self) {
        self.querying = false;

        //Request the data
        //
    }

    pub fn intersect_with_globe() {}

    pub fn render_current_intersection() {}
}
