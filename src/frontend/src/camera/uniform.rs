use super::{Camera, Projection};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    pub fn update_view_projection(camera: &Camera, projection: &Projection) -> Self {
        Self {
            view_proj: (projection.calc_matrix() * camera.calc_matrix()).to_cols_array_2d(),
        }
    }
}
