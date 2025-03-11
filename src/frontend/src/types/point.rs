use std::f32::consts::PI;
use std::hash::{Hash, Hasher};
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub coordinates: [f32; 3],
}

impl Eq for Point {}

impl Point {
    pub fn new_zero() -> Self {
        Self {
            coordinates: [0., 0., 0.],
        }
    }
    pub fn x(&self) -> f32 {
        self.coordinates[0]
    }
    pub fn y(&self) -> f32 {
        self.coordinates[1]
    }
    pub fn z(&self) -> f32 {
        self.coordinates[2]
    }

    pub fn to_array(self) -> [f32; 3] {
        self.coordinates
    }

    pub fn as_array(&self) -> &[f32; 3] {
        &self.coordinates
    }

    pub fn vec_length(&self) -> f32 {
        (self.x().powi(2) + self.y().powi(2) + self.z().powi(2)).sqrt()
    }

    // cartesian to spherical coordinates
    pub fn to_lat_lon_range(&self) -> (f32, f32, f32) {
        let lenxy = (self.x() * self.x() + self.y() * self.y()).sqrt();
        let range = self.vec_length();
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
        Self { coordinates: arr }
    }
}

impl From<&[f32; 3]> for Point {
    fn from(arr: &[f32; 3]) -> Self {
        Self { coordinates: *arr }
    }
}

impl Add for Point {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            coordinates: [
                self.coordinates[0] + other.coordinates[0],
                self.coordinates[1] + other.coordinates[1],
                self.coordinates[2] + other.coordinates[2],
            ],
        }
    }
}

impl AddAssign for Point {
    fn add_assign(&mut self, other: Self) {
        self.coordinates[0] += other.coordinates[0];
        self.coordinates[1] += other.coordinates[1];
        self.coordinates[2] += other.coordinates[2];
    }
}

impl Sub for Point {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self {
            coordinates: [
                self.coordinates[0] - other.coordinates[0],
                self.coordinates[1] - other.coordinates[1],
                self.coordinates[2] - other.coordinates[2],
            ],
        }
    }
}

impl SubAssign for Point {
    fn sub_assign(&mut self, other: Self) {
        self.coordinates[0] -= other.coordinates[0];
        self.coordinates[1] -= other.coordinates[1];
        self.coordinates[2] -= other.coordinates[2];
    }
}

impl Mul<f32> for Point {
    type Output = Self;

    fn mul(self, scalar: f32) -> Self {
        Self {
            coordinates: [
                self.coordinates[0] * scalar,
                self.coordinates[1] * scalar,
                self.coordinates[2] * scalar,
            ],
        }
    }
}

impl MulAssign<f32> for Point {
    fn mul_assign(&mut self, scalar: f32) {
        self.coordinates[0] *= scalar;
        self.coordinates[1] *= scalar;
        self.coordinates[2] *= scalar;
    }
}

impl Div<f32> for Point {
    type Output = Self;

    fn div(self, scalar: f32) -> Self {
        assert!(scalar != 0.0, "Division by zero!");
        Self {
            coordinates: [
                self.coordinates[0] / scalar,
                self.coordinates[1] / scalar,
                self.coordinates[2] / scalar,
            ],
        }
    }
}

impl DivAssign<f32> for Point {
    fn div_assign(&mut self, scalar: f32) {
        assert!(scalar != 0., "Division by zero!");
        self.coordinates[0] /= scalar;
        self.coordinates[1] /= scalar;
        self.coordinates[2] /= scalar;
    }
}

impl Hash for Point {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let combined_bits = (self.coordinates[0].to_bits() as u64)
            | ((self.coordinates[1].to_bits() as u64) << 32) // Shift to start and combine 
            ^ (self.coordinates[2].to_bits() as u64); // xor to mix

        combined_bits.hash(state);
    }
}
