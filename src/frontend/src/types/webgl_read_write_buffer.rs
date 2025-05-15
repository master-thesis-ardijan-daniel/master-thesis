use std::{io::Write, sync::Arc};

use common::{TileMetadata, TileRef};
use geo::{coord, Rect};
// use image::math::Rect;
use wasm_bindgen::UnwrapThrowExt;
use wgpu::{
    util::{self, BufferInitDescriptor, DeviceExt},
    BindGroupEntry, Buffer, BufferAddress, BufferDescriptor, BufferUsages, CommandEncoder, Device,
    Extent3d, Origin3d, Queue, RenderPass, SamplerDescriptor, ShaderStages,
    TexelCopyBufferInfoBase, TexelCopyBufferLayout, TexelCopyTextureInfo, TexelCopyTextureInfoBase,
    TextureAspect, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
    TextureViewDescriptor, VertexAttribute, VertexBufferLayout, VertexFormat, VertexStepMode,
};
use winit::event_loop::EventLoopProxy;

use crate::app::CustomEvent;

use super::{Icosphere, Point};

#[derive(Debug)]
pub struct WebGLReadWriteBuffers {
    clearing: Option<wgpu::Texture>,
    texture: wgpu::Texture,
    texture_size: Extent3d,
    texture_view: wgpu::TextureView,
    read: Buffer,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
    clearing_view: Option<wgpu::TextureView>,
    pub cpu_buffer_raw: Vec<u8>,
}

impl WebGLReadWriteBuffers {
    pub fn create(device: &Device, label: &str, size: u64, clearing_buffer: bool) -> Self {
        let size = wgpu::Extent3d {
            width: size as u32,
            height: 1,
            depth_or_array_layers: 1,
        };

        let (clearing, clearing_view) = if clearing_buffer {
            let clearing_texture = device.create_texture(&TextureDescriptor {
                label: Some(&format!("texture buffer: {label}")),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D1,
                format: TextureFormat::Rgba8Uint,
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_SRC,
                view_formats: &[],
            });

            let clearing_texture_view =
                clearing_texture.create_view(&TextureViewDescriptor::default());

            (Some(clearing_texture), Some(clearing_texture_view))
        } else {
            (None, None)
        };

        let texture = device.create_texture(&TextureDescriptor {
            label: Some(&format!("texture buffer: {label}")),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D1,
            format: TextureFormat::Rgba8Uint,
            usage: TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_DST
                | TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        let texture_view = texture.create_view(&TextureViewDescriptor::default());

        let read = device.create_buffer(&BufferDescriptor {
            label: Some(&format!("reading buffer: {label}")),
            size: 256 * 16,
            usage: BufferUsages::COPY_DST | BufferUsages::COPY_SRC | BufferUsages::UNIFORM,
            mapped_at_creation: false,
        });

        let mut entries_layout = vec![
            wgpu::BindGroupLayoutEntry {
                // GPU writing texture
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Uint,
                    view_dimension: wgpu::TextureViewDimension::D1,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderStages::NONE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ];

        let mut entries_bind_group = vec![
            BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&texture_view),
            },
            BindGroupEntry {
                binding: 1,
                resource: read.as_entire_binding(),
            },
            // BindGroupEntry { binding: 2, resource:  },
        ];

        if clearing_buffer {
            entries_layout.push(wgpu::BindGroupLayoutEntry {
                // GPU clearing texture
                binding: 2,
                visibility: ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Uint,
                    view_dimension: wgpu::TextureViewDimension::D1,
                    multisampled: false,
                },
                count: None,
            });

            entries_bind_group.push(BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(clearing_view.as_ref().unwrap()),
            });
        }

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(&format!("{label} Layout")),
            entries: &entries_layout,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(&format!("{label} Bind Group")),
            layout: &bind_group_layout,
            entries: &entries_bind_group,
        });

        Self {
            clearing,
            clearing_view,
            texture,
            texture_view,
            read,
            texture_size: size,
            bind_group_layout,
            bind_group,
            cpu_buffer_raw: vec![],
        }
    }

    pub fn clear_buffer(&self, encoder: &mut CommandEncoder) {
        let clearing_buffer = self
            .clearing
            .as_ref()
            .expect("Requested a clear, when to clearing buffer does not exist");

        let from = TexelCopyTextureInfoBase {
            texture: clearing_buffer,
            mip_level: 0,
            origin: Origin3d { x: 0, y: 0, z: 0 },
            aspect: TextureAspect::All,
        };

        let to = TexelCopyTextureInfoBase {
            texture: &self.texture,
            mip_level: 0,
            origin: Origin3d { x: 0, y: 0, z: 0 },
            aspect: TextureAspect::All,
        };

        encoder.copy_texture_to_texture(from, to, self.texture_size);
    }

    fn write_to_gpu(&self, queue: &Queue, data: &[u8]) {
        queue.write_texture(
            TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: Origin3d { x: 0, y: 0, z: 0 },
                aspect: TextureAspect::All,
            },
            data,
            TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(256 * 4),
                rows_per_image: Some(1),
            },
            self.texture_size,
        );
    }
    pub fn copy_texture_to_buffer(&self, encoder: &mut CommandEncoder) {
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: Origin3d { x: 0, y: 0, z: 0 },
                aspect: TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &self.read,
                layout: TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: None,
                    rows_per_image: None,
                },
            },
            self.texture_size,
        );
    }

    pub fn read_data<F>(
        &mut self,
        device: &Device,
        queue: &Queue,
        encoder: &mut CommandEncoder,
        eventloop: &EventLoopProxy<CustomEvent>,
        event_type: F,
    ) where
        F: Fn(Vec<u8>) -> CustomEvent + 'static,
    {
        self.copy_texture_to_buffer(encoder);

        let proxy = eventloop.clone();
        let (tx, rx) = tokio::sync::oneshot::channel();

        wgpu::util::DownloadBuffer::read_buffer(device, queue, &self.read.slice(..), move |x| {
            let data = x.unwrap();
            let data = data.to_vec();

            tx.send(data);
        });

        wasm_bindgen_futures::spawn_local(async move {
            let data = rx.await.unwrap();

            proxy.send_event(event_type(data));
        });
    }
}
// }

// pub trait SequentialRead {
//     fn gpu_read_all(
//         &self,
//         device: &Device,
//         queue: &Queue,
//         encoder: &mut CommandEncoder,
//     ) -> Vec<Vec<u8>>;
// }
// impl SequentialRead for Vec<&WebGLReadWriteBuffers> {
//     fn gpu_read_all(&self, device: &Device, queue: &Queue, encoder: &mut CommandEncoder) {
//         for buffer in self.iter() {
//             buffer.copy_texture_to_buffer(encoder);
//         }

//         {
//             for buffer in self.iter() {
//                 wgpu::util::DownloadBuffer::read_buffer(
//                     device,
//                     queue,
//                     &buffer.read.slice(..),
//                     |x| {
//                         if let Some(data) = x.ok() {
//                             results.push(data.to_vec());
//                         };
//                     },
//                 );
//             }
//         }
//     }
// }
