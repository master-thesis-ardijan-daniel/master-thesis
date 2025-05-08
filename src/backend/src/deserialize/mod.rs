use bytemuck::Pod;
use geo::{Coord, CoordNum};
use serde::Serialize;
use tree::{TileData, TileNode};

use crate::Bounds;

mod reader;
mod tree;

pub use tree::GeoTree;

pub trait Deserialize<'de> {
    fn deserialize(bytes: &'de [u8]) -> (usize, Self);
}

impl<'a, T> Deserialize<'a> for TileData<'a, T>
where
    T: Pod,
{
    fn deserialize(bytes: &'a [u8]) -> (usize, Self) {
        let mut cur = 0;

        let (read, aggregate) = Deserialize::deserialize(&bytes[cur..]);
        cur += read;

        let (read, tile) = Deserialize::deserialize(&bytes[cur..]);
        cur += read;

        (cur, Self { aggregate, tile })
    }
}

impl<'a, T> Deserialize<'a> for TileNode<'a, T>
where
    T: Pod,
{
    fn deserialize(bytes: &'a [u8]) -> (usize, Self) {
        let mut cur = 0;

        let (read, bounds) = Deserialize::deserialize(&bytes[cur..]);
        cur += read;

        let (read, children) = Deserialize::deserialize(&bytes[cur..]);
        cur += read;

        (cur, Self { bounds, children })
    }
}

#[derive(Serialize)]
pub struct TileRef<'a, T> {
    pub data: Option<Vec<&'a [T]>>,
    pub bounds: Bounds,
}

impl<'a, T: Pod> Deserialize<'a> for &'a T {
    fn deserialize(bytes: &'a [u8]) -> (usize, Self) {
        let mut cur = 0;

        let read = std::mem::size_of::<T>();
        let out = bytemuck::from_bytes::<T>(&bytes[cur..cur + read]);
        cur += read;

        (cur, out)
    }
}

impl<T> Deserialize<'_> for Coord<T>
where
    T: CoordNum + Pod,
{
    fn deserialize(bytes: &[u8]) -> (usize, Self) {
        let mut cur = 0;

        let read = std::mem::size_of::<T>();
        let x = *bytemuck::from_bytes::<T>(&bytes[cur..cur + read]);
        cur += read;

        let read = std::mem::size_of::<T>();
        let y = *bytemuck::from_bytes::<T>(&bytes[cur..cur + read]);
        cur += read;

        (cur, Self { x, y })
    }
}

impl Deserialize<'_> for Bounds {
    fn deserialize(bytes: &[u8]) -> (usize, Self) {
        let mut cur = 0;

        let (read, min) = Coord::deserialize(&bytes[cur..]);
        cur += read;

        let (read, max) = Coord::deserialize(&bytes[cur..]);
        cur += read;

        (cur, Self::new(min, max))
    }
}

impl<'a, T> Deserialize<'a> for &'a [T]
where
    T: Pod,
{
    fn deserialize(bytes: &'a [u8]) -> (usize, Self) {
        let mut cur = 0;

        let read = std::mem::size_of::<usize>();
        let len = *bytemuck::from_bytes::<usize>(&bytes[cur..cur + read]);
        cur += read;

        let read = len * std::mem::size_of::<T>();
        let array = bytemuck::cast_slice(&bytes[cur..cur + read]);
        cur += read;

        (cur, array)
    }
}

impl<'a, T> Deserialize<'a> for Vec<&'a [T]>
where
    T: Pod,
{
    fn deserialize(bytes: &'a [u8]) -> (usize, Self) {
        let mut cur = 0;

        let read = std::mem::size_of::<usize>();
        let height = *bytemuck::from_bytes::<usize>(&bytes[cur..cur + read]);
        cur += read;

        let mut out = Vec::with_capacity(height);

        for _ in 0..height {
            let (read, array) = <&[T] as Deserialize>::deserialize(&bytes[cur..]);

            out.push(array);

            cur += read;
        }

        (cur, out)
    }
}

impl<'a, T> Deserialize<'a> for Option<T>
where
    T: Deserialize<'a>,
{
    fn deserialize(bytes: &'a [u8]) -> (usize, Self) {
        let mut cur = 0;

        let read = std::mem::size_of::<usize>();
        let option = *bytemuck::from_bytes::<usize>(&bytes[cur..cur + read]);
        cur += read;

        if option != 1 {
            return (cur, None);
        }

        let (read, value) = Deserialize::deserialize(&bytes[cur..]);
        cur += read;

        (cur, Some(value))
    }
}
