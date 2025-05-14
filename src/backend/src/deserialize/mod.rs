use bytemuck::Pod;
use geo::{Coord, CoordNum};
use serde::Serialize;
use tree::{TileData, TileNode};

use crate::Bounds;

mod reader;
mod tree;

pub use tree::GeoTree;

#[derive(Clone, Copy)]
pub struct AlignedReader<'a> {
    inner: &'a [u8],
    position: usize,
}

impl<'a> AlignedReader<'a> {
    pub fn new(inner: &'a [u8]) -> Self {
        Self { inner, position: 0 }
    }

    fn read<T>(&mut self) -> &'a T
    where
        T: Pod,
    {
        let padding = self.padding::<T>();
        self.position += padding;

        let read = std::mem::size_of::<T>();
        let out = bytemuck::from_bytes(&self.inner[self.position..self.position + read]);
        self.position += read;

        out
    }

    fn read_slice<T>(&mut self, len: usize) -> &'a [T]
    where
        T: Pod,
    {
        let padding = self.padding::<T>();
        self.position += padding;

        let read = std::mem::size_of::<T>() * len;
        let out = bytemuck::cast_slice(&self.inner[self.position..self.position + read]);
        self.position += read;

        out
    }

    pub fn padding<T>(&self) -> usize {
        let alignment = std::mem::align_of::<T>();
        let remainder = self.position % alignment;

        if remainder == 0 {
            0
        } else {
            alignment - remainder
        }
    }
}

pub trait Deserialize<'de> {
    fn deserialize(bytes: &mut AlignedReader<'de>) -> Self;
}

impl<'a, T> Deserialize<'a> for TileData<'a, T>
where
    T: Pod,
{
    fn deserialize(reader: &mut AlignedReader<'a>) -> Self {
        let aggregate = Deserialize::deserialize(reader);
        let tile = Deserialize::deserialize(reader);

        Self { aggregate, tile }
    }
}

impl<'a, T> Deserialize<'a> for TileNode<'a, T>
where
    T: Pod,
{
    fn deserialize(reader: &mut AlignedReader<'a>) -> Self {
        let bounds = Deserialize::deserialize(reader);
        let children = Deserialize::deserialize(reader);

        Self { bounds, children }
    }
}

#[derive(Serialize)]
pub struct TileRef<'a, T> {
    pub data: Option<Vec<&'a [T]>>,
    pub bounds: Bounds,
}

impl<'a, T: Pod> Deserialize<'a> for &'a T {
    fn deserialize(reader: &mut AlignedReader<'a>) -> Self {
        reader.read()
    }
}

impl<T> Deserialize<'_> for Coord<T>
where
    T: CoordNum + Pod,
{
    fn deserialize(reader: &mut AlignedReader<'_>) -> Self {
        let x = *reader.read::<T>();
        let y = *reader.read::<T>();

        Self { x, y }
    }
}

impl Deserialize<'_> for Bounds {
    fn deserialize(reader: &mut AlignedReader<'_>) -> Self {
        let min = Coord::deserialize(reader);
        let max = Coord::deserialize(reader);

        Self::new(min, max)
    }
}

impl<'a, T> Deserialize<'a> for &'a [T]
where
    T: Pod,
{
    fn deserialize(reader: &mut AlignedReader<'a>) -> Self {
        let len = *reader.read::<usize>();

        reader.read_slice(len)
    }
}

impl<'a, T> Deserialize<'a> for Vec<&'a [T]>
where
    T: Pod,
{
    fn deserialize(reader: &mut AlignedReader<'a>) -> Self {
        let height = *reader.read::<usize>();

        (0..height)
            .map(|_| <&[T] as Deserialize>::deserialize(reader))
            .collect()
    }
}

impl<'a, T> Deserialize<'a> for Option<T>
where
    T: Deserialize<'a>,
{
    fn deserialize(reader: &mut AlignedReader<'a>) -> Self {
        let option = *reader.read::<u8>();

        if option != 1 {
            return None;
        }

        Some(Deserialize::deserialize(reader))
    }
}
