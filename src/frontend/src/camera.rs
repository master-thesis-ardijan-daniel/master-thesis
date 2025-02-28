use cgmath::{perspective, Matrix4, Point3, Quaternion, Rad, Rotation3, Vector3};
use web_time::Duration;
use winit::{event::*, keyboard::KeyCode};

#[derive(Debug)]
pub struct Camera {
    pub position: Point3<f32>,
    pub target: Point3<f32>,
    yaw: Rad<f32>,
    pitch: Rad<f32>,
}

impl Camera {
    pub fn new<V, Y, P>(position: V, yaw: Y, pitch: P) -> Self
    where
        V: Into<Point3<f32>>,
        Y: Into<Rad<f32>>,
        P: Into<Rad<f32>>,
    {
        Self {
            position: position.into(),
            yaw: yaw.into(),
            pitch: pitch.into(),
            target: Point3::new(0., 0., 0.),
        }
    }

    pub fn calc_matrix(&self) -> Matrix4<f32> {
        Matrix4::look_at_rh(self.position, self.target, Vector3::unit_y())
    }
}

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.5,
    0.0, 0.0, 0.0, 1.0,
);

pub struct Projection {
    aspect: f32,
    fovy: Rad<f32>,
    znear: f32,
    zfar: f32,
}

impl Projection {
    pub fn new<F>(width: u32, height: u32, fovy: F, znear: f32, zfar: f32) -> Self
    where
        F: Into<Rad<f32>>,
    {
        Self {
            aspect: width as f32 / height as f32,
            fovy: fovy.into(),
            znear,
            zfar,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.aspect = width as f32 / height as f32;
    }

    pub fn calc_matrix(&self) -> Matrix4<f32> {
        OPENGL_TO_WGPU_MATRIX * perspective(self.fovy, self.aspect, self.znear, self.zfar)
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
    target: Point3<f32>,
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
            target: Point3::new(0., 0., 0.),
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
        self.rotate_vertical = mouse_dy as f32;
    }

    pub fn update_camera(&mut self, camera: &mut Camera, duration: Duration) {
        let duration = duration.as_secs_f32();

        camera.yaw += Rad(self.rotate_horizontal) * self.sensitivity * duration;
        camera.pitch += Rad(-self.rotate_vertical) * self.sensitivity * duration;

        let scroll_factor = self.scroll * self.speed * self.sensitivity * duration;
        self.radius = (self.radius - scroll_factor).max(1.0); // Prevent zero or negative radius

        camera.position = {
            let offset = Vector3::new(0., 0., self.radius);

            let x = Quaternion::from_angle_x(camera.pitch);
            let y = Quaternion::from_angle_y(camera.yaw);

            self.target + (y * x * offset)
        };

        self.scroll = 0.;
        self.rotate_horizontal = 0.;
        self.rotate_vertical = 0.;
    }
}
