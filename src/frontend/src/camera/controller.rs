use crate::camera::uniform::CameraUniform;
use crate::AnimationState;

use super::{Camera, Projection};
use glam::Vec2;
use web_time::Duration;
use winit::dpi::PhysicalPosition;
use winit::event::MouseScrollDelta;

#[derive(Debug)]
pub struct CameraController {
    scroll: f32,
    speed: f32,
    sensitivity: f32,

    pub rotating: bool,
    pub tilting: bool,

    pub last_position: Vec2,
    pub current_position: Vec2,

    min: f32,
    max: f32,
    size: f32,

    pub projection: Projection,
    pub camera: Camera,
}

impl CameraController {
    pub fn new(
        speed: f32,
        sensitivity: f32,
        min: f32,
        max: f32,
        size: f32,
        projection: Projection,
        camera: Camera,
    ) -> Self {
        Self {
            scroll: 0.,
            speed,
            sensitivity,

            rotating: false,
            tilting: false,

            last_position: Vec2::ZERO,
            current_position: Vec2::ZERO,

            min,
            max,
            size,

            projection,
            camera,
        }
    }

    pub fn process_cursor_moved(&mut self, mouse_dx: f64, mouse_dy: f64) {
        let position = Vec2::new(mouse_dx as f32, mouse_dy as f32);

        self.current_position = position;
    }

    pub fn process_mouse_wheel(&mut self, delta: &MouseScrollDelta) {
        self.scroll = self.speed
            * (self.camera.radius / 100.).sqrt()
            * match delta {
                MouseScrollDelta::LineDelta(_, scroll) => scroll * 0.5,
                MouseScrollDelta::PixelDelta(PhysicalPosition { y: scroll, .. }) => *scroll as f32,
            };

        self.camera.angle = {
            let min = 0.;
            let max = self.camera.angle;
            let r_min = 1.;
            let r_max = 6.;
            let r = self.camera.radius.clamp(r_min, r_max);

            max - ((r - r_min) / (r_max - r_min)) * (max - min)
        };
    }

    pub fn process_drag_start(&mut self) {
        self.rotating = true;
    }

    pub fn process_drag_stop(&mut self) {
        self.rotating = false;
    }

    pub fn update_camera(&mut self, duration: Duration) -> AnimationState {
        let duration = duration.as_secs_f32();

        let scroll_factor =
            self.scroll * self.speed * (self.camera.radius / 100.).sqrt() * duration;
        self.camera.radius = (self.camera.radius - scroll_factor).clamp(self.min, self.max);

        if self.rotating {
            self.camera.rotate(
                self.last_position,
                self.current_position,
                self.sensitivity,
                self.size,
                &self.projection,
            );
        }

        if self.tilting && self.camera.radius < 1.1 {
            let sensitivity = self.sensitivity * (self.camera.radius / 1e7).sqrt() * duration;
            let factor = self.last_position.y - self.current_position.y;
            self.camera.tilt(factor * sensitivity);
        }

        self.last_position = self.current_position;
        self.scroll = 0.0;

        self.camera.animate(duration)
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.projection.size = Vec2::new(width as f32, height as f32);
    }

    pub fn update_view_projection(&self) -> CameraUniform {
        CameraUniform::update_view_projection(&self.camera, &self.projection)
    }
}
