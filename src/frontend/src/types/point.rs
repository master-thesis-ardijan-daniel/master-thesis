use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HashablePoint(Vec3);

impl Hash for HashablePoint {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let combined_bits = (self.x().to_bits() as u64)
            | ((self.y().to_bits() as u64) << 32) // Shift to start and combine
            ^ (self.z().to_bits() as u64); // xor to mix

        state.write_u64(combined_bits);
    }
}
