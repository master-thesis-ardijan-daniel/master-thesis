use bytemuck::{Pod, Zeroable};
use std::{cmp::max, path::Path};

use crate::{Dataset, Tile};
use common::Bounds;
use geo::Coord;

pub struct LightPollutionDataset {
    data: gdal::Dataset,
}

impl LightPollutionDataset {
    pub fn new<P>(path: P) -> Self
    where
        P: AsRef<Path>,
    {
        let data = gdal::Dataset::open(path).unwrap();

        Self { data }
    }
}

#[repr(C)]
#[derive(serde::Serialize, Pod, Zeroable, Clone, Copy, Default)]
pub struct LightPollutionAggregate {
    sum: f64,
    count: usize,
}

impl Dataset for LightPollutionDataset {
    type Type = f32;
    type AggregateType = LightPollutionAggregate;

    fn aggregate(values: &[Self::Type]) -> Option<Self::AggregateType> {
        let sum = values.iter().copied().map(Into::<f64>::into).sum();
        let count = values.len();

        Some(LightPollutionAggregate { sum, count })
    }

    fn aggregate2(values: &[Self::AggregateType]) -> Option<Self::AggregateType> {
        values.into_iter().copied().reduce(|mut acc, value| {
            acc.sum += value.sum;
            acc.count += value.count;

            acc
        })
    }

    fn downsample(data: &Tile<Self::Type>) -> Tile<Self::Type> {
        let scale = {
            let scale_height = data.len() / Self::TILE_SIZE as usize;
            let scale_width = data[0].len() / Self::TILE_SIZE as usize;

            max(scale_height, scale_width)
        };

        let height = data.len() / scale;
        let width = data[0].len() / scale;

        let mut output = vec![vec![0.0; width]; height];

        #[allow(clippy::needless_range_loop)]
        for i in 0..height {
            for j in 0..width {
                let mut sum = 0.;
                let mut count = 0.;

                for dy in 0..scale {
                    for dx in 0..scale {
                        let input_i = i * scale + dy;
                        let input_j = j * scale + dx;

                        if input_i < data.len() && input_j < data[0].len() {
                            let value = data[input_i][input_j];
                            if value == Self::default() {
                                continue;
                            }

                            sum += data[input_i][input_j];
                            count += 1.;
                        }
                    }
                }

                output[i][j] = sum / count;
            }
        }

        output
    }

    fn default() -> Self::Type {
        u16::MAX as f32
    }

    fn data(&self) -> Tile<Self::Type> {
        let band = self
            .data
            .rasterbands()
            .next()
            .unwrap()
            .unwrap()
            .read_band_as::<Self::Type>()
            .unwrap();

        let ((cols, _), data) = band.into_shape_and_vec();

        data.chunks(cols).map(|chunk| chunk.to_vec()).collect()
    }

    fn bounds(&self) -> Bounds {
        Bounds::new(Coord { x: -180., y: -90. }, Coord { x: 180., y: 90. })
    }

    const TILE_SIZE: u32 = 256;

    const CHILDREN_PER_AXIS: usize = 2;

    const MAX_LEVEL: u32 = 0;
}
