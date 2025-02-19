use wasm_bindgen::prelude::*;
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

#[cfg(feature = "debug")]
fn init_debug() {
    use env_logger;
    use log::Level;
    //env_logger::init();

    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
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

    let _ = window.request_inner_size(PhysicalSize::new(450, 400));

    let mut state = State::new(&window).await;
    let mut surface_configured = false;

    let _ = event_loop.run(move |event, control_flow| match event {
        // Dont use run, move over to spawn
        Event::WindowEvent { event, .. } if !state.input(&event) => match event {
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
