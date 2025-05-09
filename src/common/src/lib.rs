use bytemuck::Zeroable;
use serde::{Deserialize, Serialize};

pub type Bounds = geo::Rect<f32>;
pub type Tile<T> = Vec<Vec<T>>;

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct Coordinate {
    pub lat: f32,
    pub lon: f32,
}

#[derive(Debug, Deserialize)]
pub struct TileRef<T> {
    pub data: Tile<T>,
    pub bounds: Bounds,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TileMetadata {
    pub nw_lat: f32,
    pub nw_lon: f32,
    pub se_lat: f32,
    pub se_lon: f32,
    pub width: u32,
    pub height: u32,
    pub pad_1: u32,
    pub pad_2: u32,
}

impl<T> TileRef<T>
where
    T: Clone + Zeroable,
{
    pub fn get_padded_tile(&self, desired_width: u32, desired_height: u32) -> Vec<Vec<T>> {
        let mut tile = self.data.clone();

        tile.resize_with(desired_height as usize, || {
            vec![T::zeroed(); desired_width as usize]
        });

        for y in &mut tile {
            y.resize_with(desired_width as usize, T::zeroed);
        }

        tile
    }
}

impl<T> From<&TileRef<T>> for TileMetadata {
    fn from(tile: &TileRef<T>) -> Self {
        Self {
            nw_lat: tile.bounds.max().y,
            nw_lon: tile.bounds.min().x,
            se_lat: tile.bounds.min().y,
            se_lon: tile.bounds.max().x,
            width: tile.data[0].len() as u32,
            height: tile.data.len() as u32,
            pad_1: 0,
            pad_2: 0,
        }
    }
}
