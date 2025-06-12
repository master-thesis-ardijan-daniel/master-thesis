use winit::{
    dpi::PhysicalPosition,
    event::{KeyEvent, MouseButton, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
    window::CursorIcon,
};

use super::State;

impl State {
    pub fn input(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } => {
                if state.is_pressed() {
                    self.camera_state.controller.process_drag_start();
                } else {
                    self.camera_state.controller.process_drag_stop();
                }

                self.window.set_cursor(if state.is_pressed() {
                    CursorIcon::Grabbing
                } else {
                    CursorIcon::Grab
                });
            }

            WindowEvent::MouseInput {
                state,
                button: MouseButton::Right,
                ..
            } => {
                self.camera_state.controller.tilting = state.is_pressed();
                // self.earth_state.query_poi.start_query_poi();
                self.window.set_cursor(if state.is_pressed() {
                    CursorIcon::EResize
                } else {
                    // self.earth_state.query_poi.end_query_poi();
                    CursorIcon::Grab
                });
            }

            WindowEvent::CursorEntered { .. } => {
                self.window.set_cursor(CursorIcon::Grab);
            }

            WindowEvent::CursorLeft { .. } => {
                self.window.set_cursor(CursorIcon::Default);
            }

            WindowEvent::CursorMoved {
                position: PhysicalPosition { x, y },
                ..
            } => {
                self.camera_state.controller.process_cursor_moved(*x, *y);
                // self.earth_state.query_poi.process_cursor_moved(*x, *y);

                if self.camera_state.controller.rotating || self.camera_state.controller.tilting {
                    self.window.request_redraw();
                } else {
                    self.camera_state
                        .controller
                        .update_camera(web_time::Duration::ZERO);
                }
            }

            WindowEvent::MouseWheel { delta, .. } => {
                self.camera_state.controller.process_mouse_wheel(delta);
                self.window.request_redraw();
            }

            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state,
                        physical_key: PhysicalKey::Code(KeyCode::KeyW),
                        ..
                    },
                ..
            } => {
                if state.is_pressed() {
                    self.set_render_wireframe(!self.render_wireframe);
                    self.window.request_redraw();
                }
            }

            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state,
                        physical_key: PhysicalKey::Code(KeyCode::KeyL),
                        ..
                    },
                ..
            } => {
                if state.is_pressed() {
                    self.earth_state
                        .set_render_lp_map(!self.earth_state.render_lp_map, &self.queue);
                    self.window.request_redraw();
                }
            }
            _ => {}
        }
    }
}
