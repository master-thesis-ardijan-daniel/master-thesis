use std::{fs::File, io::Result, path::Path};

use geo::{Coord, Intersects, Rect};
use serialize::{AlignedWriter, Serialize};

pub mod deserialize;
pub mod serialize;

pub mod earth_map;
// mod light_pollution;
pub mod population;

fn slice<D>(data: &Tile<D::Type>, x: usize, y: usize, width: usize, height: usize) -> Tile<D::Type>
where
    D: Dataset,
    D::Type: Clone,
{
    let mut result = Vec::with_capacity(height);

    for source_row in data.iter().take((y + height).min(data.len())).skip(y) {
        let mut row = Vec::with_capacity(width);

        for column in source_row
            .iter()
            .take((x + width).min(source_row.len()))
            .skip(x)
        {
            row.push(column.clone());
        }

        while row.len() < width {
            row.push(source_row.last().cloned().unwrap_or_else(D::default));
        }

        result.push(row);
    }

    while result.len() < height {
        if let Some(last_row) = result.last() {
            result.push(last_row.clone());
        } else {
            result.push(vec![D::default(); width]);
        }
    }

    result
}

pub type Tile<T> = Vec<Vec<T>>;
pub type Bounds = Rect<f32>;

pub trait Dataset {
    type Type;
    type AggregateType;

    fn aggregate(_values: &[Self::Type]) -> Option<Self::AggregateType> {
        None
    }
    fn aggregate2(_values: &[Self::AggregateType]) -> Option<Self::AggregateType> {
        None
    }

    fn downsample(data: &Tile<Self::Type>) -> Tile<Self::Type>;
    fn default() -> Self::Type;

    fn data(&self) -> Tile<Self::Type>;
    fn bounds(&self) -> Bounds;

    const TILE_SIZE: u32;
    const CHILDREN_PER_AXIS: usize;
    const MAX_LEVEL: u32;
}

fn flatten<T>(data: Vec<Vec<&Tile<T>>>) -> Tile<T>
where
    T: Clone,
{
    let mut result = Vec::new();

    for outer_row in 0..data.len() {
        for inner_row in 0..data[0][0].len() {
            let mut row = Vec::new();

            for outer_col in 0..data[0].len() {
                row.extend_from_slice(&data[outer_row][outer_col][inner_row]);
            }

            result.push(row);
        }
    }

    result
}

pub struct TileNode<T, U> {
    pub bounds: Bounds,
    pub data: Option<Tile<T>>,
    pub aggregate: Option<U>,

    pub children: Vec<Vec<TileNode<T, U>>>,
}

pub struct GeoTree<D>
where
    D: Dataset,
{
    pub root: TileNode<D::Type, D::AggregateType>,
}

#[derive(serde::Serialize)]
pub struct TileRef<'a, T, U> {
    pub bounds: &'a Bounds,
    pub data: Option<&'a Tile<T>>,
    pub aggregate: Option<&'a U>,
}

impl<'a, T, U> From<&'a TileNode<T, U>> for TileRef<'a, T, U> {
    fn from(tile: &'a TileNode<T, U>) -> Self {
        Self {
            bounds: &tile.bounds,
            data: tile.data.as_ref(),
            aggregate: tile.aggregate.as_ref(),
        }
    }
}

impl<D> GeoTree<D>
where
    D: Dataset,
{
    pub fn get_tiles(
        &self,
        area: Bounds,
        level: u32,
    ) -> Vec<TileRef<'_, D::Type, D::AggregateType>> {
        fn inner<T, U>(
            level: u32,
            current_level: u32,
            node: &TileNode<T, U>,
            area: Bounds,
        ) -> Option<Vec<TileRef<'_, T, U>>> {
            if node.bounds.intersects(&area) {
                if current_level == level {
                    return Some(vec![node.into()]);
                }

                Some(
                    node.children
                        .iter()
                        .flatten()
                        .flat_map(|child| inner(level, current_level + 1, child, area))
                        .flatten()
                        .collect(),
                )
            } else {
                None
            }
        }

        inner(level, 0, &self.root, area).unwrap()
    }
}

impl<D> GeoTree<D>
where
    D: Dataset,
    D::Type: Copy,
    D::AggregateType: Copy,
{
    pub fn build(data: &D) -> Self {
        let mut root = TileNode {
            bounds: data.bounds(),
            data: None,
            aggregate: None,
            children: Vec::new(),
        };

        Self::recursive_slice(&mut root, data.data());
        Self::propagate(&mut root);

        Self { root }
    }

    fn propagate(parent: &mut TileNode<D::Type, D::AggregateType>) {
        if parent.children.is_empty() {
            return;
        }

        for child in parent.children.iter_mut().flatten() {
            Self::propagate(child);
        }

        let data = parent
            .children
            .iter()
            .map(|row| row.iter().flat_map(|child| &child.data).collect::<Vec<_>>())
            .collect::<Vec<_>>();

        let data = flatten(data);

        parent.data = Some(D::downsample(&data));

        let aggregates = parent
            .children
            .iter()
            .flatten()
            .flat_map(|child| &child.aggregate)
            .copied()
            .collect::<Vec<_>>();

        parent.aggregate = D::aggregate2(&aggregates);
    }

    fn recursive_slice(parent: &mut TileNode<D::Type, D::AggregateType>, data: Tile<D::Type>) {
        let height = data.len();
        let width = data[0].len();

        if height as u32 <= D::TILE_SIZE && width as u32 <= D::TILE_SIZE {
            parent.aggregate = D::aggregate(&data.iter().flatten().copied().collect::<Vec<_>>());
            parent.data = Some(data);

            return;
        }

        let child_width = width / D::CHILDREN_PER_AXIS;
        let child_height = height / D::CHILDREN_PER_AXIS;

        parent.children = (0..D::CHILDREN_PER_AXIS)
            .map(|i| {
                (0..D::CHILDREN_PER_AXIS)
                    .map(|j| {
                        let x_start = j * child_width;
                        let y_start = i * child_height;

                        let actual_width = if j == D::CHILDREN_PER_AXIS - 1 {
                            width - x_start
                        } else {
                            child_width
                        };

                        let actual_height = if i == D::CHILDREN_PER_AXIS - 1 {
                            height - y_start
                        } else {
                            child_height
                        };

                        let child_data =
                            slice::<D>(&data, x_start, y_start, actual_width, actual_height);

                        let bounds_delta_w = parent.bounds.width() / D::CHILDREN_PER_AXIS as f32;
                        let bounds_delta_h = parent.bounds.height() / D::CHILDREN_PER_AXIS as f32;
                        let bounds = Rect::new(
                            Coord {
                                x: parent.bounds.min().x + (j as f32 * bounds_delta_w),
                                y: parent.bounds.max().y - (i as f32 * bounds_delta_h),
                            },
                            Coord {
                                x: parent.bounds.min().x + ((j + 1) as f32 * bounds_delta_w),
                                y: parent.bounds.max().y - ((i + 1) as f32 * bounds_delta_h),
                            },
                        );

                        let mut child = TileNode {
                            bounds,
                            data: None,
                            aggregate: None,
                            children: Vec::new(),
                        };

                        Self::recursive_slice(&mut child, child_data);

                        child
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
    }

    pub fn write_to_file<P>(&self, path: P) -> Result<()>
    where
        P: AsRef<Path>,
        TileNode<D::Type, D::AggregateType>: Serialize,
    {
        let Ok(file) = File::create_new(path) else {
            return Ok(());
        };

        let mut writer = AlignedWriter::new(&file);
        self.root.serialize(&mut writer)?;

        Ok(())
    }
}
