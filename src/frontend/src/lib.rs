use std::{cell::RefCell, rc::Rc, sync::Arc};

use types::PerformanceMetrics;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::js_sys;
use web_time::Instant;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::*,
    event_loop::{ActiveEventLoop, EventLoop},
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

// #[wasm_bindgen(start)]
// pub async fn run() {
//     let mut perf_metrics = PerformanceMetrics::new();
//     #[cfg(feature = "debug")]
//     init_debug();

//     let event_loop = EventLoop::new().unwrap();
//     let window = WindowBuilder::new()
//         .build(&event_loop)
//         .expect("Could not create window");

//     #[cfg(target_arch = "wasm32")]
//     {
//         let map_element = web_sys::window()
//             .and_then(|w| w.document())
//             .and_then(|doc| doc.get_element_by_id("map_canvas"))
//             .expect("retrieved map element");

//         map_element
//             .append_child(&web_sys::Element::from(
//                 window.canvas().expect("created a canvas"),
//             ))
//             .expect("added canvas to map element");
//     }

//     let _ = window.request_inner_size(PhysicalSize::new(1000, 1000));

//     let mut state = State::new(&window).await;
//     let mut surface_configured = false;

//     let mut last_render = Instant::now();

//     let _ = event_loop.spawn_app(move |event, control_flow| match event {
//         // Dont use run, move over to spawn
//         Event::WindowEvent { event, .. } if !state.input(&event) => match event {
//             WindowEvent::RedrawRequested => {
//                 state.window().request_redraw();
//                 if !surface_configured {
//                     return;
//                 }

//                 let now = Instant::now();
//                 state.delta = now - last_render;
//                 last_render = now;

//                 if let Some(v) = safe_get_subdivision_level() {
//                     state.earth_state.set_subdivision_level(v);
//                 }

//                 state.update();
//                 perf_metrics.time_new_frame();
//                 perf_metrics.send_perf_event();

//                 match state.render() {
//                     Ok(_) => {}
//                     Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
//                         state.resize(state.size)
//                     }

//                     #[cfg(feature = "debug")]
//                     Err(wgpu::SurfaceError::OutOfMemory) => {
//                         log::error!("Out of Memory!!!");
//                         control_flow.exit();
//                     }

//                     #[cfg(feature = "debug")]
//                     Err(wgpu::SurfaceError::Timeout) => {
//                         log::warn!("Surface timeout")
//                     }

//                     _ => {}
//                 }
//             }

//             WindowEvent::Resized(new_size) => {
//                 surface_configured = true;
//                 state.resize(new_size);
//             }
//             WindowEvent::CloseRequested
//             | WindowEvent::KeyboardInput {
//                 event:
//                     KeyEvent {
//                         state: ElementState::Pressed,
//                         physical_key: PhysicalKey::Code(KeyCode::Escape),
//                         ..
//                     },
//                 ..
//             } => control_flow.exit(),
//             _ => {}
//         },
//         _ => {}
//     });
// }

struct App {
    state: Option<State>,
}

impl App {
    pub fn new() -> Self {
        Self { state: None }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attributes = WindowAttributes::default()
            .with_title("Learn WGPU")
            .with_inner_size(PhysicalSize::new(1000, 1000));
        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        #[cfg(target_arch = "wasm32")]
        {
            use winit::platform::web::WindowExtWebSys;
            web_sys::window()
                .and_then(|win| win.document())
                .and_then(|doc| {
                    let dst = doc.get_element_by_id("wasm-example")?;
                    let canvas = web_sys::Element::from(window.canvas()?);
                    dst.append_child(&canvas).ok()?;
                    Some(())
                })
                .expect("Couldn't append canvas to document body.");
        }

        //Fugly but idk what else to do...
        let app_ref = Rc::new(RefCell::new(std::mem::replace(self, App::new())));
        spawn_local({
            let self_local = Rc::clone(&app_ref);
            async move {
                let state = State::new(window).await;
                self_local.borrow_mut().state = Some(state);
            }
        });
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested
            | WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state: ElementState::Pressed,
                        physical_key: PhysicalKey::Code(KeyCode::Escape),
                        ..
                    },
                ..
            } => event_loop.exit(),
            WindowEvent::RedrawRequested => {
                if let Some(state) = self.state.as_mut() {
                    state.window.request_redraw();
                }
            }
            _ => {}
        }
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub fn run() {
    #[cfg(target_arch = "wasm32")]
    {
        let event_loop = EventLoop::new().unwrap();

        let mut app = App::new();

        use winit::platform::web::EventLoopExtWebSys;
        event_loop.spawn_app(app);
    }
}
