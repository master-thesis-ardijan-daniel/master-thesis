use winit::{
    dpi::PhysicalPosition,
    event::{MouseButton, WindowEvent},
    window::CursorIcon,
};

use super::State;

impl State {
    pub fn input(&mut self, event: &WindowEvent) -> bool {
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
                true
            }

            WindowEvent::CursorEntered { .. } => {
                self.window.set_cursor(CursorIcon::Grab);
                true
            }

            WindowEvent::CursorLeft { .. } => {
                self.window.set_cursor(CursorIcon::Default);
                true
            }

            WindowEvent::CursorMoved {
                position: PhysicalPosition { x, y },
                ..
            } => {
                self.camera_state.controller.process_cursor_moved(*x, *y);

                true
            }

            WindowEvent::MouseWheel { delta, .. } => {
                self.camera_state.controller.process_mouse_wheel(delta);
                true
            }

            _ => false,
        }
    }
}
