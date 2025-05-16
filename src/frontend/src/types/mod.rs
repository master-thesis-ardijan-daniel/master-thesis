pub mod icosphere;
pub use icosphere::*;
pub mod earth;
pub use earth::*;
pub mod performance_metrics;
pub use performance_metrics::*;

use std::f32::consts::PI;
use std::hash::{Hash, Hasher};
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign};

use glam::{Vec3, Vec3Swizzles};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HashablePoint(Vec3);

impl Eq for HashablePoint {}

impl Hash for HashablePoint {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let combined_bits = (self.0.x.to_bits() as u64)
            | ((self.0.y.to_bits() as u64) << 32) // Shift to start and combine
            ^ (self.0.z.to_bits() as u64); // xor to mix

        state.write_u64(combined_bits);
    }
}

impl From<Vec3> for HashablePoint {
    fn from(value: Vec3) -> Self {
        HashablePoint(value)
    }
}
