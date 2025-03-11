use std::f32::consts::PI;
use std::hash::{Hash, Hasher};
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign};

use glam::Vec3;

#[derive(Clone, Copy, PartialEq)]
pub struct Point(Vec3);

impl Point {
    pub const ZERO: Self = Self(Vec3::ZERO);

    pub fn x(&self) -> f32 {
        self.0.x
    }

    pub fn y(&self) -> f32 {
        self.0.y
    }

    pub fn z(&self) -> f32 {
        self.0.z
    }

    pub fn to_array(&self) -> [f32; 3] {
        self.0.to_array()
    }

    // cartesian to spherical coordinates
    pub fn to_lat_lon_range(&self) -> (f32, f32, f32) {
        let lenxy = (self.x() * self.x() + self.y() * self.y()).sqrt();
        let range = self.0.length();
        if lenxy < 1.0e-10 {
            if self.z() > 0. {
                return (PI / 2., 0.0, range);
            }
            (-(PI / 2.), 0.0, range)
        } else {
            let lat = self.z().atan2(lenxy);
            let lon = self.y().atan2(self.x());
            (lat, lon, range)
        }
    }
}

impl From<[f32; 3]> for Point {
    fn from(arr: [f32; 3]) -> Self {
        Self(Into::into(arr))
    }
}

impl From<&[f32; 3]> for Point {
    fn from(arr: &[f32; 3]) -> Self {
        Self(Into::into(*arr))
    }
}

impl Add for Point {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self(self.0 + other.0)
    }
}

impl AddAssign for Point {
    fn add_assign(&mut self, other: Self) {
        self.0 += other.0;
    }
}

impl Sub for Point {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self(self.0 - other.0)
    }
}

impl SubAssign for Point {
    fn sub_assign(&mut self, other: Self) {
        self.0 -= other.0
    }
}

impl Mul<f32> for Point {
    type Output = Self;

    fn mul(self, scalar: f32) -> Self {
        Self(self.0 * scalar)
    }
}

impl MulAssign<f32> for Point {
    fn mul_assign(&mut self, scalar: f32) {
        self.0 *= scalar;
    }
}

impl Div<f32> for Point {
    type Output = Self;

    fn div(self, scalar: f32) -> Self {
        assert!(scalar != 0.0, "Division by zero!");

        Self(self.0 / scalar)
    }
}

impl DivAssign<f32> for Point {
    fn div_assign(&mut self, scalar: f32) {
        assert!(scalar != 0., "Division by zero!");

        self.0 /= scalar;
    }
}

impl Eq for Point {}

impl Hash for Point {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let combined_bits = (self.x().to_bits() as u64)
            | ((self.y().to_bits() as u64) << 32) // Shift to start and combine
            ^ (self.z().to_bits() as u64); // xor to mix

        state.write_u64(combined_bits);
    }
}
