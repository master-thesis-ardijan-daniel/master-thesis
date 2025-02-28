use glam::{Mat4, Vec2};

pub struct Projection {
    pub size: Vec2,
    pub fovy: f32,
    znear: f32,
    zfar: f32,
}

impl Projection {
    pub fn new(width: u32, height: u32, fovy: f32, znear: f32, zfar: f32) -> Self {
        Self {
            size: Vec2::new(width as f32, height as f32),
            fovy,
            znear,
            zfar,
        }
    }

    pub fn aspect(&self) -> f32 {
        self.size.x / self.size.y
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.size = Vec2::new(width as f32, height as f32);
    }

    pub fn calc_matrix(&self) -> Mat4 {
        Mat4::perspective_lh(self.fovy, self.aspect(), self.znear, self.zfar)
    }
}
