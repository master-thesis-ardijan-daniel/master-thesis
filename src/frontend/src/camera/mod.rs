use std::f32::consts::{FRAC_1_SQRT_2, FRAC_PI_2, SQRT_2};

use glam::{Mat3, Mat4, Quat, Vec2, Vec3, Vec3Swizzles};

mod controller;
pub use controller::CameraController;

mod projection;
pub use projection::Projection;

mod state;
pub use state::CameraState;

use crate::AnimationState;

mod uniform;

const R: f32 = 0.8;

#[derive(Debug)]
pub struct Camera {
    pub radius: f32,
    current_radius: f32,

    pub angle: f32,
    current_angle: f32,

    pub orientation: Quat,
    current_orientation: Quat,

    angular_velocity: Quat,
    friction: f32,
}

impl Camera {
    pub fn new(radius: f32) -> Self {
        Self {
            radius,
            current_radius: radius,

            angle: 0.,
            current_angle: 0.,

            orientation: Quat::IDENTITY,
            current_orientation: Quat::IDENTITY,

            angular_velocity: Quat::IDENTITY,
            friction: 5.,
        }
    }

    pub fn calc_matrix(&self) -> Mat4 {
        Mat4::from_rotation_x(self.current_angle)
            * Mat4::from_translation(Vec3::Z * self.current_radius)
            * Mat4::from_quat(self.current_orientation)
    }

    pub fn tilt(&mut self, delta: f32) {
        self.angle = (self.angle + delta).clamp(0., FRAC_PI_2);
    }

    pub fn rotate(
        &mut self,
        previous_position: Vec2,
        current_position: Vec2,
        sensitivity: f32,
        object_radius: f32,
        projection: &Projection,
    ) {
        let visible_height = 2.0 * self.current_radius * (projection.fovy * 0.5).tan();

        let object_screen_radius = (object_radius / visible_height) * projection.size.min_element();

        let screen_center = projection.size * 0.5;

        let p1 = Vec2::new(-1., 1.) * (previous_position - screen_center) / object_screen_radius;
        let p2 = Vec2::new(-1., 1.) * (current_position - screen_center) / object_screen_radius;

        let transformation = Mat3::from_rotation_x(-self.angle);

        let p1 = (transformation * p1.extend(0.)).xy();
        let p2 = (transformation * p2.extend(0.)).xy();

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

        let impulse = Quat::from_axis_angle(axis, angle * sensitivity);

        self.angular_velocity = (impulse * self.angular_velocity).normalize();
    }

    pub fn animate(&mut self, duration: f32) -> AnimationState {
        let mut animation_state = AnimationState::Finished;

        if !self.angular_velocity.is_near_identity() {
            let friction_factor = (-self.friction * duration).exp();

            let (axis, angle) = self.angular_velocity.to_axis_angle();

            let frame_rotation = Quat::from_axis_angle(axis, angle * duration);

            self.current_orientation = (frame_rotation * self.current_orientation).normalize();

            let (axis, angle) = self.angular_velocity.to_axis_angle();

            self.angular_velocity = Quat::from_axis_angle(axis, angle * friction_factor);

            if self.angular_velocity.is_near_identity() {
                self.angular_velocity = Quat::IDENTITY;
            }

            animation_state = AnimationState::Animating;
        }

        if self.radius != self.current_radius {
            self.current_radius += -(self.current_radius - self.radius) * self.friction * duration;

            if (self.radius - self.current_radius).abs() < 1e-3 {
                self.current_radius = self.radius;
            }

            animation_state = AnimationState::Animating;
        }

        if self.angle != self.current_angle {
            self.current_angle += -(self.current_angle - self.angle) * self.friction * duration;

            if (self.angle - self.current_angle).abs() < 1e-3 {
                self.current_angle = self.angle;
            }

            animation_state = AnimationState::Animating;
        }

        animation_state
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
