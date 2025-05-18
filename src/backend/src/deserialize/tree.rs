use std::path::Path;

use bytemuck::{Pod, Zeroable};
use common::{Bounds, TileRefResponse};
use geo::Intersects;

use crate::{deserialize::reader::Reader, Dataset};

pub struct GeoTree<D>
where
    D: Dataset,
{
    data: memmap2::Mmap,
    _dataset: std::marker::PhantomData<D>,
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
        let reader = Reader::new(&self.data);

        fn inner<'a, T, U>(
            level: u32,
            current_level: u32,
            pointer: &Pointer<T>,
            area: Bounds,
            reader: &Reader<'a>,
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

        inner::<_, D::AggregateType>(level, 0, &Pointer::default(), area, &reader).unwrap()
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
