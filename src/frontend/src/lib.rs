use wasm_bindgen::prelude::*;
use web_sys::js_sys;
use web_time::Instant;
use winit::{
    dpi::PhysicalSize,
    event::*,
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::WindowBuilder,
};

#[cfg(target_arch = "wasm32")]
use winit::platform::web::WindowExtWebSys;

pub mod state;
pub mod types;
pub use state::*;
pub mod camera;

#[cfg(feature = "debug")]
fn init_debug() {
    use env_logger;
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

struct PerformanceMetrics {
    total_frame_time: f64,
    number_of_frames: f64,
    highest_frame_time: f32,
    lowest_frame_time: f32,
    startup_time: f32,
    timer_since_last_frame: Instant,
    timer_since_last_reset: Instant,
}

impl PerformanceMetrics {
    pub fn new() -> Self {
        Self {
            total_frame_time: 0.,
            highest_frame_time: -1.,
            lowest_frame_time: 100000.,
            startup_time: -1.,
            number_of_frames: 0.,
            timer_since_last_frame: web_time::Instant::now(),
            timer_since_last_reset: web_time::Instant::now(),
        }
    }

    fn time_new_frame(&mut self) {
        let frame_time = self.timer_since_last_frame.elapsed().as_secs_f64();
        self.total_frame_time += frame_time;
        self.number_of_frames += 1.;
        self.timer_since_last_frame = web_time::Instant::now();

        let frame_time = frame_time as f32;

        // Startup
        if self.startup_time < 0. {
            self.startup_time = frame_time;
            return;
        }

        if self.highest_frame_time < frame_time {
            self.highest_frame_time = frame_time;
        }

        if self.lowest_frame_time > frame_time {
            self.lowest_frame_time = frame_time;
        }
    }

    fn send_perf_event(&mut self) {
        if self.timer_since_last_reset.elapsed().as_secs_f64() < 0.1 {
            return;
        }

        let event_data = js_sys::Map::new();
        let js_str = |s: &str| JsValue::from_str(s);
        let js_f32 = |s: f32| JsValue::from_f64(s as f64);

        event_data.set(
            &js_str("avg_frame_time"),
            &js_f32(self.get_avg_frame_time()),
        );
        event_data.set(
            &js_str("highest_frame_time"),
            &js_f32(self.highest_frame_time),
        );
        event_data.set(
            &js_str("lowest_frame_time"),
            &js_f32(self.lowest_frame_time),
        );
        event_data.set(&js_str("startup_time"), &js_f32(self.startup_time));

        handle_new_perf_data(event_data);
        self.reset();
    }

    fn reset(&mut self) {
        self.total_frame_time = 0.;
        self.highest_frame_time = -1.;
        self.lowest_frame_time = 1000000.;
        self.number_of_frames = 0.;
        self.timer_since_last_frame = web_time::Instant::now();
        self.timer_since_last_reset = web_time::Instant::now();
    }

    fn get_avg_frame_time(&self) -> f32 {
        if self.number_of_frames == 0. {
            return 0.;
        }
        (self.total_frame_time / self.number_of_frames) as f32
    }
}

#[wasm_bindgen(start)]
pub async fn run() {
    let mut perf_metrics = PerformanceMetrics::new();
    #[cfg(feature = "debug")]
    init_debug();

    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new()
        .build(&event_loop)
        .expect("Could not create window");

    #[cfg(target_arch = "wasm32")]
    {
        let map_element = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|doc| doc.get_element_by_id("map_canvas"))
            .expect("retrieved map element");

        map_element
            .append_child(&web_sys::Element::from(
                window.canvas().expect("created a canvas"),
            ))
            .expect("added canvas to map element");
    }

    let _ = window.request_inner_size(PhysicalSize::new(1000, 1000));

    let mut state = State::new(&window).await;
    let mut surface_configured = false;

    let mut last_render = Instant::now();

    let _ = event_loop.run(move |event, control_flow| match event {
        // Dont use run, move over to spawn
        Event::DeviceEvent {
            event: DeviceEvent::MouseMotion { delta },
            ..
        } => {
            if state.mouse_pressed {
                state.camera_controller.process_mouse(delta.0, delta.1);
            }
        }

        Event::WindowEvent { event, .. } if !state.input(&event) => match event {
            WindowEvent::RedrawRequested => {
                state.window().request_redraw();
                if !surface_configured {
                    return;
                }

                let now = Instant::now();
                let duration = now - last_render;
                last_render = now;
                state.update(duration);
                perf_metrics.time_new_frame();
                perf_metrics.send_perf_event();

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
