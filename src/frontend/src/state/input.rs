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
                self.camera_state.controller.rotating = state.is_pressed();
                self.window.set_cursor(if state.is_pressed() {
                    CursorIcon::Grabbing
                } else {
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

                if self.camera_state.controller.rotating {
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

            _ => {}
        }
    }
}
