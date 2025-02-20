use std::collections::HashMap;
use std::thread;
use web_time::{Duration, Instant};

use gloo::net::http::Request;
use wasm_bindgen::convert::OptionFromWasmAbi;
use wasm_bindgen::prelude::*;
use web_sys::js_sys::{self, Uint8Array};
use wgpu::{core::present::ResolvedSurfaceOutput, util::RenderEncoder, FragmentState, Surface};
use winit::dpi::PhysicalSize;
use winit::{
    event::*,
    event_loop::{EventLoop, EventLoopWindowTarget},
    keyboard::{KeyCode, PhysicalKey},
    platform::web::WindowExtWebSys,
    window::{Window, WindowBuilder},
};

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys;

pub mod state;
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

// Hente bilde fra url
// Bruk fetch
// helst async (sjekk i ettertid at kjøretid ikke eksploderer)
// binde et stort buffer, som kan brukes til å bytte ut teksutrer.
//
//
pub async fn download_image(url: &str) -> Vec<u8> {
    Request::get(url)
        .send()
        .await
        .unwrap()
        .binary()
        .await
        .unwrap()
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

    let _ = window.request_inner_size(PhysicalSize::new(450, 400));

    let mut state = State::new(&window).await;
    let mut surface_configured = false;

    let mut texture_buffer: Vec<Vec<u8>> = vec![];

    let image_data = download_image("hello_world.png").await;

    let mut last_render = Instant::now();

    let _ = event_loop.run(move |event, control_flow| match event {
        // Dont use run, move over to spawn
        Event::WindowEvent { event, window_id } if !state.input(&event) => match event {
            WindowEvent::RedrawRequested => {
                state.window().request_redraw();
                if !surface_configured {
                    return;
                }

                let now = Instant::now();
                let duration = now - last_render;
                last_render = now;
                state.update(duration);

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

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
}
