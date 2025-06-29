use std::sync::Arc;

use common::TileResponse;
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
    CreateState(Box<State>),
    HttpResponse(CustomResponseType),
}

#[derive(Debug)]
pub enum CustomResponseType {
    SatelliteImage(TileResponse<[u8; 4]>, (u32, u32, u32)),
    Population(TileResponse<f32>, (u32, u32, u32)),
    LightPollution(TileResponse<f32>, (u32, u32, u32)),
}

pub struct App {
    state: Option<State>,
    perf_metrics: PerformanceMetrics,
    #[allow(dead_code)]
    proxy_eventloop: EventLoopProxy<CustomEvent>,
}

impl App {
    pub fn _new(proxy: EventLoopProxy<CustomEvent>) -> Self {
        Self {
            state: None,
            perf_metrics: PerformanceMetrics::new(),
            proxy_eventloop: proxy,
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

            let proxy_eventloop = self.proxy_eventloop.clone();
            wasm_bindgen_futures::spawn_local(async move {
                proxy_eventloop
                    .clone()
                    .send_event(CustomEvent::CreateState(Box::new(
                        State::new(_window, proxy_eventloop).await,
                    )))
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

            (event, Some(state)) => {
                state.input(&event);
                state.touch(&event);
            }

            _ => {}
        }
    }

    // Workaround because State::new needs to be async
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: CustomEvent) {
        match (event, &mut self.state) {
            (CustomEvent::CreateState(state), None) => {
                self.state = Some(*state);
            }

            (
                CustomEvent::HttpResponse(CustomResponseType::SatelliteImage(tile, id)),
                Some(state),
            ) => {
                state.earth_state.update_tile_buffer = true;
                state.earth_state.insert_tile(id, tile);
                state.window.request_redraw();
            }
            // (
            //     CustomEvent::HttpResponse(CustomResponseType::PopulationTileResponse(tile, id)),
            //     Some(state),
            // ) => {
            //     state.earth_state.update_tile_buffer = true;
            //     state.earth_state.insert_population_tile(id, tile);
            //     state.window.request_redraw();
            // }
            (
                CustomEvent::HttpResponse(CustomResponseType::LightPollution(tile, id)),
                Some(state),
            ) => {
                state.earth_state.update_tile_buffer = true;
                state.earth_state.insert_lp_tile(id, tile);
                state.window.request_redraw();
            }
            _ => {}
        }
    }
}
