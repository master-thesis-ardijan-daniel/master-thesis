use std::f32::consts::{FRAC_1_SQRT_2, SQRT_2};

use glam::{Mat4, Quat, Vec2, Vec3};

mod controller;
pub use controller::CameraController;

mod projection;
pub use projection::Projection;

mod state;
pub use state::CameraState;

mod uniform;

const R: f32 = 0.8;

#[derive(Debug)]
pub struct Camera {
    pub orientation: Quat,
    pub radius: f32,
}

impl Camera {
    pub fn new(radius: f32) -> Self {
        Self {
            orientation: Quat::IDENTITY,
            radius,
        }
    }

    pub fn calc_matrix(&self) -> Mat4 {
        Mat4::from_translation(Vec3::Z * self.radius) * Mat4::from_quat(self.orientation)
    }

    pub fn rotate(
        &mut self,
        previous_position: Vec2,
        current_position: Vec2,
        sensitivity: f32,
        object_radius: f32,
        projection: &Projection,
    ) {
        let visible_height = 2.0 * self.radius * (projection.fovy * 0.5).tan();

        let object_screen_radius = (object_radius / visible_height) * projection.size.min_element();

        let screen_center = projection.size * 0.5;

        let p1 = Vec2::new(-1., 1.) * (previous_position - screen_center) / object_screen_radius;
        let p2 = Vec2::new(-1., 1.) * (current_position - screen_center) / object_screen_radius;

        let p2 = {
            let movement = p2 - p1;

            p1 + movement * sensitivity
        };

        let v1 = Self::point_to_sphere(p1);
        let v2 = Self::point_to_sphere(p2);

        let axis = v1.cross(v2);

        if axis.length_squared() < 1e-10 {
            return;
        }

        let axis = axis.normalize();

        let angle = {
            let distance = Vec2::distance(p1, p2) / (2. * R);

            2. * distance.clamp(-1., 1.).asin()
        };

        let rotation = Quat::from_axis_angle(axis, angle);

        self.orientation = (rotation * self.orientation).normalize();
    }
    fn point_to_sphere(point @ Vec2 { x, y }: Vec2) -> Vec3 {
        let d = point.length_squared();

        let z = if d < R * FRAC_1_SQRT_2 {
            (R * R - d * d).sqrt()
        } else {
            let t = R / SQRT_2;
            t * t / d
        };

        Vec3::new(x, y, z).normalize()
    }
}
