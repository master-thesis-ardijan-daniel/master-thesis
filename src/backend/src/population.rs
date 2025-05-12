use std::cmp::max;

use backend::{Bounds, Dataset};
use geo::Coord;

pub struct PopulationDataset {
    data: gdal::Dataset,
}

impl PopulationDataset {
    pub fn new<P>(path: P) -> Self
    where
        P: AsRef<std::path::Path>,
    {
        let data = gdal::Dataset::open(path).unwrap();

        Self { data }
    }
}

impl Dataset for PopulationDataset {
    type Type = f32;
    type AggregateType = f64;

    fn aggregate(values: &[Self::Type]) -> Option<Self::AggregateType> {
        Some(
            values
                .iter()
                .filter(|&&c| c >= Self::default())
                .map(|&x| x as f64)
                .sum(),
        )
    }

    fn aggregate2(values: &[Self::AggregateType]) -> Option<Self::AggregateType> {
        Some(
            values
                .iter()
                .filter(|&&c| c >= (Self::default() as f64))
                .sum(),
        )
    }

    fn downsample(data: &backend::Tile<Self::Type>) -> backend::Tile<Self::Type> {
        let scale = {
            let scale_height = data.len() / Self::TILE_SIZE as usize;
            let scale_width = data[0].len() / Self::TILE_SIZE as usize;

            max(scale_height, scale_width)
        };

        let height = data.len() / scale;
        let width = data[0].len() / scale;

        let mut output = vec![vec![0.; width]; height];

        for i in 0..height {
            for j in 0..width {
                let mut sum = 0.0;

                // Sum over the block
                for dy in 0..scale {
                    for dx in 0..scale {
                        let input_i = i * scale + dy;
                        let input_j = j * scale + dx;

                        if input_i < data.len() && input_j < data[0].len() {
                            sum += data[input_i][input_j];
                        }
                    }
                }

                output[i][j] = sum;
            }
        }

        output
    }

    fn default() -> Self::Type {
        -3.402823e+38
    }

    fn data(&self) -> backend::Tile<Self::Type> {
        let band = self
            .data
            .rasterbands()
            .next()
            .unwrap()
            .unwrap()
            .read_band_as::<Self::Type>()
            .unwrap();

        let ((cols, _rows), data) = band.into_shape_and_vec();

        data.chunks(cols).map(|chunk| chunk.to_vec()).collect()
    }

    fn bounds(&self) -> backend::Bounds {
        Bounds::new(Coord { x: -180., y: -90. }, Coord { x: 180., y: 90. })
    }

    const TILE_SIZE: u32 = 256;

    const CHILDREN_PER_AXIS: usize = 2;

    const MAX_LEVEL: u32 = 11;
}
