use std::sync::Arc;

use types::PerformanceMetrics;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::js_sys;
use web_time::Instant;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::*,
    event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy},
    keyboard::{KeyCode, PhysicalKey},
    window::WindowAttributes,
};

#[cfg(target_arch = "wasm32")]
use winit::platform::web::WindowExtWebSys;

pub mod state;
pub mod types;
pub use state::*;
pub mod camera;

#[cfg(feature = "debug")]
fn init_debug() {
    // use env_logger;
    use log::Level;
    //env_logger::init();

    console_error_panic_hook::set_once();
    console_log::init_with_level(Level::Warn).expect("Unable to init console_log");
}

#[wasm_bindgen]
extern "C" {
    fn handle_new_perf_data(data: js_sys::Map);

    fn get_subdivision_level() -> js_sys::Number;
}

pub fn safe_get_subdivision_level() -> Option<usize> {
    let n: f64 = get_subdivision_level().value_of();
    // #[cfg(feature = "debug")]
    // log::warn!("subdiv_level is {:?}", n);
    if n.is_nan() {
        return None;
    }

    if n < 0. {
        return Some(0);
    }

    Some(n as usize)
}

struct App {
    state: Option<State>,
    last_render: web_time::Instant,
    perf_metrics: PerformanceMetrics,
    proxy: Arc<EventLoopProxy<CustomEvent>>,
}

impl App {
    pub fn new(proxy: EventLoopProxy<CustomEvent>) -> Self {
        Self {
            perf_metrics: PerformanceMetrics::new(),
            state: None,
            last_render: web_time::Instant::now(),
            proxy: Arc::new(proxy),
        }
    }
}

impl ApplicationHandler<CustomEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attributes = WindowAttributes::default()
            .with_title("Lets test WASM!")
            .with_inner_size(PhysicalSize::new(1000, 1000));
        let window = Arc::new(
            event_loop
                .create_window(window_attributes)
                .expect("Could not create window!"),
        );

        #[cfg(target_arch = "wasm32")]
        {
            use winit::platform::web::WindowExtWebSys;
            web_sys::window()
                .and_then(|win| win.document())
                .and_then(|doc| {
                    let dst = doc.get_element_by_id("map_canvas")?;
                    let canvas = web_sys::Element::from(window.canvas()?);
                    dst.append_child(&canvas).ok()?;
                    Some(())
                })
                .expect("Couldn't append canvas to document body.");
        }

        let proxy = self.proxy.clone();
        spawn_local(async move {
            proxy
                .send_event(CustomEvent::CreateState(State::new(window).await))
                .unwrap();
        });
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        if let Some(state) = self.state.as_mut() {
            if state.input(&event) {
                return;
            }
        }

        match (event, self.state.is_some()) {
            (WindowEvent::RedrawRequested, true) => {
                let state = self.state.as_mut().expect("State should exist");
                state.window().request_redraw();

                let now = Instant::now();
                state.delta = now - self.last_render;
                self.last_render = now;

                if let Some(v) = safe_get_subdivision_level() {
                    state.earth_state.set_subdivision_level(v);
                }

                state.update();
                self.perf_metrics.time_new_frame();
                self.perf_metrics.send_perf_event();

                match state.render() {
                    Ok(_) => {}
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        state.resize(state.size)
                    }

                    #[cfg(feature = "debug")]
                    Err(wgpu::SurfaceError::OutOfMemory) => {
                        log::error!("Out of Memory!!!");
                        event_loop.exit();
                    }

                    #[cfg(feature = "debug")]
                    Err(wgpu::SurfaceError::Timeout) => {
                        log::warn!("Surface timeout")
                    }

                    _ => {}
                }
            }

            (WindowEvent::Resized(new_size), true) => {
                self.state.as_mut().unwrap().resize(new_size);
            }
            (
                WindowEvent::CloseRequested
                | WindowEvent::KeyboardInput {
                    event:
                        KeyEvent {
                            state: ElementState::Pressed,
                            physical_key: PhysicalKey::Code(KeyCode::Escape),
                            ..
                        },
                    ..
                },
                _,
            ) => event_loop.exit(),
            _ => {}
        }
    }

    // Workaround because State::new needs to be async
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: CustomEvent) {
        match event {
            CustomEvent::CreateState(state) => {
                self.state = Some(state);
            }
        }
    }
}

#[derive(Debug)]
enum CustomEvent {
    CreateState(State),
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub async fn run() {
    let event_loop = EventLoop::<CustomEvent>::with_user_event().build().unwrap();
    let proxy_event_loop = event_loop.create_proxy();

    let mut _app = App::new(proxy_event_loop);

    #[cfg(target_arch = "wasm32")]
    {
        init_debug();
        use winit::platform::web::EventLoopExtWebSys;
        event_loop.spawn_app(_app);
    }
}
