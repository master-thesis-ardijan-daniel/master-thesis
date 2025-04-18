use std::sync::Arc;

use web_time::Duration;
use winit::{
    application::ApplicationHandler,
    event::{StartCause, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoopProxy},
    window::WindowAttributes,
};

use crate::{safe_get_subdivision_level, types::PerformanceMetrics, State};

#[derive(Debug)]
pub enum CustomEvent {
    CreateState(State),
}

pub struct App {
    state: Option<State>,
    perf_metrics: PerformanceMetrics,
    proxy: EventLoopProxy<CustomEvent>,
}

impl App {
    pub fn new(proxy: EventLoopProxy<CustomEvent>) -> Self {
        Self {
            perf_metrics: PerformanceMetrics::new(),
            state: None,
            proxy,
        }
    }
}

impl ApplicationHandler<CustomEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attributes = WindowAttributes::default()
            .with_title("Lets test WASM!")
            .with_maximized(true);

        let _window = Arc::new(
            event_loop
                .create_window(window_attributes)
                .expect("Could not create window!"),
        );

        #[cfg(target_arch = "wasm32")]
        {
            use winit::platform::web::WindowExtWebSys;

            let map_element = web_sys::window()
                .and_then(|w| w.document())
                .and_then(|doc| doc.get_element_by_id("map_canvas"))
                .expect("retrieved map element");

            map_element
                .append_child(&web_sys::Element::from(
                    _window.canvas().expect("created a canvas"),
                ))
                .expect("added canvas to map element");

            let proxy = self.proxy.clone();
            wasm_bindgen_futures::spawn_local(async move {
                proxy
                    .send_event(CustomEvent::CreateState(State::new(_window).await))
                    .unwrap();
            });
        }
    }

    fn new_events(&mut self, _event_loop: &ActiveEventLoop, cause: StartCause) {
        let Some(state) = self.state.as_mut() else {
            return;
        };

        if let StartCause::WaitCancelled {
            start,
            requested_resume,
        } = cause
        {
            state.delta = requested_resume
                .map(|r| r.duration_since(start))
                .unwrap_or(Duration::from_millis(20));
        }
    }

    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match (event, &mut self.state) {
            (WindowEvent::RedrawRequested, Some(state)) => {
                if let Some(v) = safe_get_subdivision_level() {
                    state.earth_state.set_subdivision_level(v);
                }

                state.update();

                self.perf_metrics.time_new_frame();
                self.perf_metrics.send_perf_event();

                match state.render() {
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        state.window.request_redraw();
                        state.resize(state.size)
                    }

                    #[cfg(feature = "debug")]
                    Err(wgpu::SurfaceError::OutOfMemory) => {
                        log::error!("Out of Memory!!!");
                        _event_loop.exit();
                    }

                    #[cfg(feature = "debug")]
                    Err(wgpu::SurfaceError::Timeout) => {
                        log::warn!("Surface timeout")
                    }

                    _ => {}
                }
            }

            (WindowEvent::Resized(new_size), Some(state)) => {
                state.resize(new_size);
                state.window.request_redraw();
            }

            (event, Some(state)) => state.input(&event),

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
