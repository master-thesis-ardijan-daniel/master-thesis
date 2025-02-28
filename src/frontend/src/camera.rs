

use glam::{Mat4, Quat, Vec3};
use web_time::Duration;
use winit::{event::*, keyboard::KeyCode};

#[derive(Debug)]
pub struct Camera {
    pub position: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    yaw: f32,
    pitch: f32,
}

impl Camera {
    pub fn new(position: Vec3, yaw: f32, pitch: f32) -> Self {
        Self {
            position,
            yaw,
            pitch,
            target: Vec3::ZERO,
            up: Vec3::Y,
        }
    }

    pub fn calc_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.position, self.target, self.up)
    }
}

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: Mat4 = Mat4::from_cols_array(&[
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.5,
    0.0, 0.0, 0.0, 1.0,
]);

pub struct Projection {
    aspect: f32,
    fovy: f32,
    znear: f32,
    zfar: f32,
}

impl Projection {
    pub fn new(width: u32, height: u32, fovy: f32, znear: f32, zfar: f32) -> Self {
        Self {
            aspect: width as f32 / height as f32,
            fovy,
            znear,
            zfar,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.aspect = width as f32 / height as f32;
    }

    pub fn calc_matrix(&self) -> Mat4 {
        OPENGL_TO_WGPU_MATRIX
            * Mat4::perspective_rh_gl(self.fovy, self.aspect, self.znear, self.zfar)
    }
}

#[derive(Debug)]
pub struct CameraController {
    amount_left: f32,
    amount_right: f32,
    amount_forward: f32,
    amount_backward: f32,
    amount_up: f32,
    amount_down: f32,
    rotate_horizontal: f32,
    rotate_vertical: f32,
    radius: f32,
    scroll: f32,
    speed: f32,
    sensitivity: f32,
}

impl CameraController {
    pub fn new(speed: f32, sensitivity: f32) -> Self {
        Self {
            amount_left: 0.,
            amount_right: 0.,
            amount_forward: 0.,
            amount_backward: 0.,
            amount_up: 0.,
            amount_down: 0.,
            rotate_horizontal: 0.,
            rotate_vertical: 0.,
            scroll: 0.,
            speed,
            sensitivity,
            radius: 20.,
        }
    }

    pub fn process_keyboard(&mut self, key: KeyCode, state: ElementState) -> bool {
        let amount = if state == ElementState::Pressed {
            1.
        } else {
            0.
        };

        match key {
            KeyCode::KeyW | KeyCode::ArrowUp => {
                self.amount_forward = amount;
                true
            }
            KeyCode::KeyS | KeyCode::ArrowDown => {
                self.amount_backward = amount;
                true
            }
            KeyCode::KeyA | KeyCode::ArrowLeft => {
                self.amount_left = amount;
                true
            }
            KeyCode::KeyD | KeyCode::ArrowRight => {
                self.amount_right = amount;
                true
            }
            KeyCode::Space => {
                self.amount_up = amount;
                true
            }
            KeyCode::ShiftLeft => {
                self.amount_down = amount;
                true
            }
            _ => false,
        }
    }

    pub fn process_mouse(&mut self, mouse_dx: f64, mouse_dy: f64) {
        self.rotate_horizontal = mouse_dx as f32;
        self.rotate_vertical = -mouse_dy as f32;
    }

    pub fn update_camera(&mut self, camera: &mut Camera, duration: Duration) {
        let duration = duration.as_secs_f32();

        camera.yaw += self.rotate_horizontal * self.sensitivity * duration;
        camera.pitch += -self.rotate_vertical * self.sensitivity * duration;

        let scroll_factor = self.scroll * self.speed * self.sensitivity * duration;
        self.radius = (self.radius - scroll_factor).max(1.0); // Prevent zero or negative radius

        camera.pitch = camera.pitch.max(-SAFE_FRAC_PI_2).min(SAFE_FRAC_PI_2);

        camera.position = {
            let offset = Vec3::new(0., 0., self.radius);

            // Apply yaw first (around world y-axis)
            let y_rotation = Quat::from_axis_angle(Vec3::Y, camera.yaw);
            let rotated_offset = y_rotation * offset;

            // Then apply pitch (around local x-axis)
            // First establish the right vector after yaw rotation
            let right = y_rotation * Vec3::X;

            // Now rotate around that right vector for pitch
            let x_rotation = Quat::from_axis_angle(right, camera.pitch);
            let final_offset = x_rotation * rotated_offset;

            camera.target + final_offset
        };

        self.scroll = 0.;
        self.rotate_horizontal = 0.;
        self.rotate_vertical = 0.;
    }
}
