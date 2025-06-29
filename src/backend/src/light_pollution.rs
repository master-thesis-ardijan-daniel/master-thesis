use bytemuck::{Pod, Zeroable};
use std::path::Path;

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
        values.iter().copied().reduce(|mut acc, value| {
            acc.sum += value.sum;
            acc.count += value.count;

            acc
        })
    }

    fn downsample(data: &Tile<Self::Type>) -> Tile<Self::Type> {
        let input_height = data.len();
        let input_width = data[0].len();

        let output_height = Self::TILE_SIZE as usize;
        let output_width = Self::TILE_SIZE as usize;

        let scale_y = input_height as f32 / output_height as f32;
        let scale_x = input_width as f32 / output_width as f32;

        let mut output = vec![vec![0.0; output_width]; output_height];

        #[allow(clippy::needless_range_loop)]
        for out_y in 0..output_height {
            for out_x in 0..output_width {
                let y0 = (out_y as f32 * scale_y).floor() as usize;
                let y1 = ((out_y + 1) as f32 * scale_y)
                    .ceil()
                    .min(input_height as f32) as usize;

                let x0 = (out_x as f32 * scale_x).floor() as usize;
                let x1 = ((out_x + 1) as f32 * scale_x)
                    .ceil()
                    .min(input_width as f32) as usize;

                let mut sum = 0.0;
                let mut count = 0.0;

                for y in y0..y1 {
                    for x in x0..x1 {
                        sum += data[y][x];
                        count += 1.0;
                    }
                }

                output[out_y][out_x] = if count > 0.0 { sum / count } else { 0.0 };
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
