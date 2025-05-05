use wasm_bindgen::prelude::*;
use web_sys::js_sys;

pub mod state;
pub mod types;
pub use state::*;
mod app;
pub mod camera;
mod tiles;

#[cfg(feature = "debug")]
fn init_debug() {
    use log::Level;

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
    if n.is_nan() {
        return None;
    }

    if n < 0. {
        return Some(0);
    }

    Some(n as usize)
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub async fn run() {
    use app::{App, CustomEvent};
    use winit::{event_loop::EventLoop, platform::web::EventLoopExtWebSys};

    let event_loop = EventLoop::<CustomEvent>::with_user_event().build().unwrap();
    let proxy_event_loop = event_loop.create_proxy();
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);

    let app = App::new(proxy_event_loop);

    #[cfg(feature = "debug")]
    init_debug();

    event_loop.spawn_app(app);
}
