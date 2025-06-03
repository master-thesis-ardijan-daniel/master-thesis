use web_time::Duration;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindingType, Buffer, BufferBindingType, BufferUsages, Device, Queue, RenderPass, ShaderStages,
};
use winit::dpi::PhysicalSize;

use crate::AnimationState;

use super::{Camera, CameraController, Projection};

#[derive(Debug)]
pub struct CameraState {
    pub controller: CameraController,

    pub bind_group_layout: BindGroupLayout,
    bind_group: BindGroup,
    buffer: Buffer,
}

impl CameraState {
    pub fn create(device: &Device, size: &PhysicalSize<u32>) -> Self {
        let controller = {
            let projection =
                Projection::new(size.width, size.height, 45.0_f32.to_radians(), 0.0001, 100.);
            let camera = Camera::new(2.);

            CameraController::new(1., 0.65, 1.2, 50., 1., projection, camera)
        };

        let buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("camera_buffer"),
            contents: bytemuck::cast_slice(&[controller.update_view_projection()]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("camera_bind_group_layout"),
        });

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        });

        Self {
            controller,
            bind_group_layout,
            bind_group,
            buffer,
        }
    }

    pub fn update(&mut self, queue: &Queue, delta: Duration) -> AnimationState {
        let animation_state = self.controller.update_camera(delta);

        let uniform = self.controller.update_view_projection();
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[uniform]));

        animation_state
    }

    pub fn render(&mut self, render_pass: &mut RenderPass<'_>) -> u32 {
        render_pass.set_bind_group(0, &self.bind_group, &[]);

        0
    }
}
