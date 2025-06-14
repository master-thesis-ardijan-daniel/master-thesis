use std::path::Path;

use bytemuck::{Pod, Zeroable};
use common::{Bounds, TileRefResponse};
use geo::{Contains, Intersects};

use crate::{deserialize::reader::Reader, Dataset};

use super::iterators::ContainsIterator;

pub struct GeoTree<D>
where
    D: Dataset,
{
    data: memmap2::Mmap,
    _dataset: std::marker::PhantomData<fn() -> D>,
}

impl<D> GeoTree<D>
where
    D: Dataset,
{
    pub fn new<P>(path: P) -> std::io::Result<Self>
    where
        P: AsRef<Path>,
    {
        let file = std::fs::File::open(path)?;
        let data = unsafe { memmap2::Mmap::map(&file)? };

        Ok(Self {
            data,
            _dataset: Default::default(),
        })
    }

    pub fn get_tiles(&self, area: Bounds, level: u32) -> Vec<TileRefResponse<'_, D::Type>>
    where
        D::Type: Pod,
        D::AggregateType: Pod,
    {
        let mut reader = Reader::new(&self.data);

        fn inner<'a, T, U>(
            level: u32,
            current_level: u32,
            pointer: &Pointer<T>,
            area: Bounds,
            reader: &mut Reader<'a>,
        ) -> Option<Vec<TileRefResponse<'a, T>>>
        where
            T: Pod,
            U: Pod,
        {
            let node = reader.load(pointer);

            if node.bounds.intersects(&area) {
                if current_level == level {
                    let data = reader.read::<TileData<T, U>>();

                    return data.tile.map(|tile| {
                        vec![TileRefResponse {
                            bounds: node.bounds,
                            data: tile,
                        }]
                    });
                }

                Some(
                    node.children
                        .iter()
                        .copied()
                        .flatten()
                        .flat_map(|child| {
                            inner::<_, U>(level, current_level + 1, child, area, reader)
                        })
                        .flatten()
                        .collect(),
                )
            } else {
                None
            }
        }

        inner::<_, D::AggregateType>(level, 0, &Pointer::default(), area, &mut reader).unwrap()
    }

    pub fn get_tile(&self, x: usize, y: usize, z: usize) -> Option<TileRefResponse<'_, D::Type>>
    where
        D::Type: Pod,
        D::AggregateType: Pod,
    {
        let mut reader = Reader::new(&self.data);

        let mut current = reader.read::<TileNode<D::Type>>();

        let max = D::CHILDREN_PER_AXIS.pow(z as u32);

        if y >= max || x >= max {
            return None;
        }

        for level in 1..=z {
            let bit_position = z - level;

            let row = (y >> bit_position) & 1;
            let col = (x >> bit_position) & 1;

            if let Some(child) = current.children.get(row).and_then(|row| row.get(col)) {
                current = reader.load(child);
            } else {
                return None;
            }
        }

        let data = reader.read::<TileData<D::Type, D::AggregateType>>();

        Some(TileRefResponse {
            data: data.tile.unwrap(),
            bounds: current.bounds,
        })
    }

    pub fn get_aggregate<Query>(&self, query: Query) -> Option<D::AggregateType>
    where
        D::Type: Pod,
        D::AggregateType: Pod,
        Query: Contains<Bounds> + Intersects<Bounds>,
    {
        let reader = Reader::new(&self.data);

        let iter = ContainsIterator::<D::Type, D::AggregateType, Query>::new(reader, query);

        iter.fold(None, |acc, data| match (acc, data.aggregate) {
            (Some(acc), Some(&aggregate)) => D::aggregate2(&[acc, aggregate]),
            (Some(acc), None) => Some(acc),
            (None, Some(&aggregate)) => Some(aggregate),
            _ => None,
        })
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, Pod, Zeroable, Debug)]
pub struct Pointer<T> {
    pub position: usize,
    _type: std::marker::PhantomData<T>,
}

impl<T> Default for Pointer<T> {
    fn default() -> Self {
        Self {
            position: 0,
            _type: Default::default(),
        }
    }
}

#[derive(Debug)]
pub struct TileNode<'a, T> {
    pub bounds: Bounds,
    pub children: Vec<&'a [Pointer<T>]>,
}

#[derive(Debug)]
pub struct TileData<'a, T, U> {
    #[allow(dead_code)]
    pub aggregate: Option<&'a U>,
    pub tile: Option<Vec<&'a [T]>>,
}
