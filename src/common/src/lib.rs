use bytemuck::Zeroable;
use serde::{Deserialize, Serialize};

pub type Bounds = geo::Rect<f32>;
pub type Tile<T> = Vec<Vec<T>>;
pub type TileRef<'a, T> = Vec<&'a [T]>;

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct Coordinate {
    pub lat: f32,
    pub lon: f32,
}

#[derive(Debug, Deserialize)]
pub struct TileResponse<T> {
    pub data: Tile<T>,
    pub bounds: Bounds,
}

#[derive(Debug, Serialize)]
pub struct TileRefResponse<'a, T> {
    pub data: TileRef<'a, T>,
    pub bounds: Bounds,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TileMetadata {
    pub nw_lat: f32,
    pub nw_lon: f32,
    pub se_lat: f32,
    pub se_lon: f32,
    pub width: u32,
    pub height: u32,
    pub level: u32,
    pub data_type: u32,
}

impl<T> TileResponse<T>
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

impl<T> From<(&TileResponse<T>, u32, u32)> for TileMetadata {
    fn from((tile, level, data_type): (&TileResponse<T>, u32, u32)) -> Self {
        Self {
            nw_lat: tile.bounds.max().y,
            nw_lon: tile.bounds.min().x,
            se_lat: tile.bounds.min().y,
            se_lon: tile.bounds.max().x,
            width: tile.data[0].len() as u32,
            height: tile.data.len() as u32,
            level,
            data_type: data_type,
        }
    }
}
