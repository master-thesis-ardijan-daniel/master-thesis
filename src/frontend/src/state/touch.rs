use std::collections::HashMap;

use winit::event::{MouseScrollDelta, Touch, TouchPhase, WindowEvent};

use super::State;

#[derive(Default, Debug)]
pub(super) struct TouchState {
    touches: HashMap<u64, (f64, f64)>,
    distance: Option<f64>,
}

impl State {
    pub fn touch(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::Touch(Touch {
                phase: TouchPhase::Started,
                location,
                id,
                ..
            }) => {
                self.camera_state.controller.rotating = false;

                self.touch_state
                    .touches
                    .insert(*id, (location.x, location.y));
            }

            WindowEvent::Touch(Touch {
                phase: TouchPhase::Moved,
                location,
                id,
                ..
            }) => {
                if self.touch_state.touches.len() == 1 {
                    self.camera_state.controller.rotating = true;
                }

                self.touch_state
                    .touches
                    .insert(*id, (location.x, location.y));
            }

            WindowEvent::Touch(Touch {
                phase: TouchPhase::Ended,
                id,
                ..
            }) => {
                self.touch_state.touches.remove(id);

                self.camera_state.controller.rotating = false;

                if self.touch_state.touches.len() < 2 {
                    self.touch_state.distance = None;
                }
            }

            _ => return,
        };

        self.window.request_redraw();

        if self.touch_state.touches.len() == 2 {
            let touch_points: Vec<_> = self.touch_state.touches.values().collect();
            let (x1, y1) = touch_points[0];
            let (x2, y2) = touch_points[1];

            let distance = ((x2 - x1).powi(2) + (y2 - y1).powi(2)).sqrt();

            if let Some(last_distance) = self.touch_state.distance {
                let zoom_factor = distance / last_distance;

                if (zoom_factor - 1.0).abs() > 0.01 {
                    let distance = if zoom_factor > 1.0 {
                        distance
                    } else {
                        -distance
                    };

                    self.camera_state
                        .controller
                        .process_mouse_wheel(&MouseScrollDelta::LineDelta(0., distance as f32));
                }
            }

            self.touch_state.distance = Some(distance);
        }

        if self.touch_state.touches.len() == 1 {
            let touch_point = self.touch_state.touches.values().next().unwrap();

            self.camera_state
                .controller
                .process_cursor_moved(touch_point.0, touch_point.1);
        }
    }
}
