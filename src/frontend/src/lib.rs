use wasm_bindgen::prelude::*;
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

#[wasm_bindgen(start)]
pub async fn run() {
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
