use wasm_bindgen::prelude::*;
use wgpu::{core::present::ResolvedSurfaceOutput, Surface};
use winit::{
    event::*,
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
    platform::web::WindowExtWebSys,
    window::{Window, WindowBuilder},
};

#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);
}

#[wasm_bindgen(start)]
pub async fn run() {
    #[cfg(feature = "debug")]
    init_debug();

    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new()
        .build(&event_loop)
        .expect("Could not create window");

    web_sys::window()
        .and_then(|w| w.document())
        .and_then(|doc| {
            let map_div = doc.get_element_by_id("map_canvas")?;
            map_div
                .append_child(&web_sys::Element::from(window.canvas()?))
                .ok()?;
            Some(())
        })
        .expect("Unable to create canvas");

    use winit::dpi::PhysicalSize;
    let _ = window.request_inner_size(PhysicalSize::new(450, 400));

    let mut state = State::new(&window).await;
    let mut surface_configured = false;
    alert("IRAN");

    let _ = event_loop.run(move |event, control_flow| match event {
        // Dont use run, move over to spawn
        Event::WindowEvent { event, window_id } if !state.input(&event) => match event {
            WindowEvent::RedrawRequested => {
                state.window().request_redraw();
                if !surface_configured {
                    return;
                }

                state.update();
                match state.render() {
                    Ok(_) => {}
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        state.resize(state.size)
                    }

                    #[cfg(feature = "debug")]
                    Err(wgpu::SurfaceError::OutOfMemory) => {
                        log::error!("Out of Memory!!!");
                        control_flow.exit();
                    }

                    #[cfg(feature = "debug")]
                    Err(wgpu::SurfaceError::Timeout) => {
                        log::warn!("Surface timeout")
                    }

                    _ => {}
                }
            }

            WindowEvent::Resized(new_size) => {
                surface_configured = true;
                state.resize(new_size);
            }
            WindowEvent::CloseRequested
            | WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state: ElementState::Pressed,
                        physical_key: PhysicalKey::Code(KeyCode::Escape),
                        ..
                    },
                ..
            } => control_flow.exit(),
            _ => {}
        },
        _ => {}
    });
}

struct State<'a> {
    surface: wgpu::Surface<'a>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    window: &'a Window,
}

impl<'a> State<'a> {
    async fn new(window: &'a Window) -> State<'a> {
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
                    required_features: wgpu::Features::empty(), // Add any genericlly available features.
                    required_limits: wgpu::Limits::downlevel_webgl2_defaults(),
                    ..Default::default()
                },
                None,
            )
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);

        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        //assert!(size.width != 0, "Width cannot be 0");
        //assert!(size.height != 0, "Width cannot be 0");

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

        Self {
            surface,
            device,
            queue,
            config,
            size,
            window,
        }
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    fn input(&mut self, event: &WindowEvent) -> bool {
        false
    }

    fn update(&mut self) {
        //todo!()
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
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
            let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Main render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

// use version 29 of winit then

#[cfg(feature = "debug")]
fn init_debug() {
    use env_logger;
    use log::Level;
    //env_logger::init();

    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    console_log::init_with_level(Level::Warn).expect("Unable to init console_log");
}
