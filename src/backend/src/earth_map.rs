use crate::{Bounds, Dataset, Tile};
use geo::Coord;
use image::{DynamicImage, ImageBuffer, ImageReader, Rgba};
use std::{fs::File, io::BufReader};

pub struct EarthmapDataset {
    data: DynamicImage,
}

impl EarthmapDataset {
    pub fn new<P>(path: P) -> Self
    where
        P: AsRef<std::path::Path>,
    {
        let file = File::open(path).unwrap();
        let reader = BufReader::new(file);

        let mut reader = ImageReader::new(reader).with_guessed_format().unwrap();

        reader.no_limits();

        let decoder = reader.into_decoder().unwrap();

        let image = DynamicImage::from_decoder(decoder).unwrap();

        EarthmapDataset { data: image }
    }
}

type Pixel = [u8; 4];

impl Dataset for EarthmapDataset {
    type Type = Pixel;
    type AggregateType = ();

    fn downsample(data: &Tile<Pixel>) -> Tile<Pixel> {
        let pixels: Vec<_> = data.iter().flatten().flatten().copied().collect();

        let image: ImageBuffer<_, Vec<_>> = image::imageops::resize(
            &ImageBuffer::<Rgba<u8>, _>::from_raw(data[0].len() as u32, data.len() as u32, pixels)
                .unwrap(),
            Self::TILE_SIZE,
            Self::TILE_SIZE,
            image::imageops::FilterType::Triangle,
        );

        image
            .rows()
            .map(|row| row.map(|pixel| pixel.0).collect())
            .collect()
    }

    fn default() -> Pixel {
        [0; 4]
    }

    fn data(&self) -> Tile<Pixel> {
        self.data
            .to_rgba8()
            .rows()
            .map(|row| row.map(|pixel| pixel.0).collect())
            .collect()
    }

    fn bounds(&self) -> Bounds {
        Bounds::new(
            Coord { x: -180.0, y: 90.0 }, // Northwest
            Coord { x: 180.0, y: -90.0 }, // Southeast
        )
    }

    const TILE_SIZE: u32 = 256;
    const CHILDREN_PER_AXIS: usize = 2;
    const MAX_LEVEL: u32 = 2;
}
