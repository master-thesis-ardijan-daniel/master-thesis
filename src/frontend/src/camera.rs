use glam::{Mat4, Quat, Vec3};
use web_time::Duration;
use winit::{dpi::PhysicalPosition, event::*, keyboard::KeyCode};

#[derive(Debug)]
pub struct Camera {
    pub position: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    pub orientation: Quat,
    pub radius: f32,
}

impl Camera {
    pub fn new(position: Vec3, yaw: f32, pitch: f32) -> Self {
        Self {
            position,
            target: Vec3::ZERO,
            up: Vec3::Y,
            orientation: Quat::IDENTITY,
            radius: 100.,
        }
    }

    pub fn calc_matrix(&self) -> Mat4 {
        let offset = self.orientation * Vec3::new(0., 0., self.radius);

        let rotation_mat = Mat4::from_quat(self.orientation);

        let eye_pos = -(self.target - offset);
        let translation = Vec3::new(
            eye_pos.dot(rotation_mat.x_axis.truncate()),
            eye_pos.dot(rotation_mat.y_axis.truncate()),
            eye_pos.dot(rotation_mat.z_axis.truncate()),
        );

        Mat4::from_cols(
            rotation_mat.col(0),
            rotation_mat.col(1),
            rotation_mat.col(2),
            translation.extend(1.0),
        )
    }
}

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
        Mat4::perspective_lh(self.fovy, self.aspect, self.znear, self.zfar)
    }
}

#[derive(Debug)]
pub struct CameraController {
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
            rotate_horizontal: 0.,
            rotate_vertical: 0.,
            scroll: 0.,
            speed,
            sensitivity,
            radius: 100.,
        }
    }

    pub fn process_keyboard(&mut self, key: KeyCode, state: ElementState) -> bool {
        false
    }

    pub fn process_mouse(&mut self, mouse_dx: f64, mouse_dy: f64) {
        self.rotate_horizontal = -mouse_dx as f32;
        self.rotate_vertical = -mouse_dy as f32;
    }

    pub fn process_scroll(&mut self, delta: &MouseScrollDelta) {
        self.scroll = match delta {
            // I'm assuming a line is about 100 pixels
            MouseScrollDelta::LineDelta(_, scroll) => scroll * 100.0,
            MouseScrollDelta::PixelDelta(PhysicalPosition { y: scroll, .. }) => *scroll as f32,
        };
    }

    pub fn update_camera(&mut self, camera: &mut Camera, duration: Duration) {
        let duration = duration.as_secs_f32();

        let adjusted_sensitivity = self.sensitivity * (self.radius / 100.).sqrt();

        //
        let right = (camera.orientation * Vec3::X).normalize();
        let up = (camera.orientation * Vec3::Y).normalize();

        // Create a rotation axis in camera-local space based on mouse input
        // This axis is perpendicular to the direction of intended rotation
        let rotation_axis =
            (right * -self.rotate_vertical + up * -self.rotate_horizontal).normalize();

        // Calculate rotation angle magnitude from mouse movement
        let rotation_angle = (self.rotate_horizontal * self.rotate_horizontal
            + self.rotate_vertical * self.rotate_vertical)
            .sqrt()
            * adjusted_sensitivity
            * duration;

        // Create a single quaternion rotation around this axis
        let rotation = if rotation_angle.abs() > 0.0001 {
            Quat::from_axis_angle(rotation_axis, rotation_angle)
        } else {
            Quat::IDENTITY
        };

        // Apply the rotation to the current orientation
        camera.orientation = rotation * camera.orientation;

        // Normalize to prevent drift
        camera.orientation = camera.orientation.normalize();

        let scroll_factor = self.scroll * self.speed * self.sensitivity * duration;
        self.radius = (self.radius - scroll_factor).clamp(20., 100.); // Prevent zero or negative radius
        camera.radius = self.radius;

        self.scroll = 0.;
        self.rotate_horizontal = 0.;
        self.rotate_vertical = 0.;
    }
}
