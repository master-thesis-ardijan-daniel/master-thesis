use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct Coordinate {
    pub lat: f32,
    pub lon: f32,
}

pub type Tile<T> = Vec<Vec<T>>;

#[derive(Clone)]
pub struct TileNode<T> {
    pub bounds: Bounds,
    pub children: Vec<Vec<TileNode<T>>>, //Should allways be smaller in dimensions than self.tile
    pub tile: Tile<T>,
    aggregate: T,
}

impl<T> TileNode<T> {
    fn get_tile(&self) -> &[Vec<T>] {
        &self.tile
    }
}

impl<T> TileNode<T> {
    fn new_parent_from_children<F>(nodes: Vec<Vec<TileNode<T>>>) -> Self
    where
        T: Clone,
        F: Dataset<T>,
    {
        let mut bounds = Bounds {
            north_west: Coordinate {
                lat: f32::MIN,
                lon: f32::MAX,
            },
            south_east: Coordinate {
                lat: f32::MAX,
                lon: f32::MIN,
            },
        };

        for node in nodes.iter().flatten() {
            bounds.north_west.lat = bounds.north_west.lat.max(node.bounds.north_west.lat);
            bounds.north_west.lon = bounds.north_west.lon.min(node.bounds.north_west.lon);
            bounds.south_east.lat = bounds.south_east.lat.min(node.bounds.south_east.lat);
            bounds.south_east.lon = bounds.south_east.lon.max(node.bounds.south_east.lon);
        }

        Self {
            aggregate: F::aggregate(
                nodes
                    .clone()
                    .into_iter()
                    .flatten()
                    .map(|node| node.aggregate)
                    .collect(),
            ),
            bounds,
            tile: todo!(),
            children: nodes,
        }
    }
}

pub struct GeoTree<T> {
    pub root: TileNode<T>,
    pub depth: usize,
    // indices: Vec<LayerIndex>,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct Bounds {
    pub north_west: Coordinate,
    pub south_east: Coordinate,
}

impl Bounds {
    pub fn contains(&self, coordinate: &Coordinate) -> bool {
        let lat = self.north_west.lat >= coordinate.lat && coordinate.lat >= self.south_east.lat;
        let lon = self.north_west.lon <= coordinate.lon && coordinate.lon <= self.south_east.lon;

        return lat && lon;
    }

    pub fn intersects(&self, area: &Bounds) -> bool {
        if self.north_west.lon > area.south_east.lon || area.south_east.lat > self.north_west.lat {
            return false;
        }

        if self.south_east.lon < area.north_west.lon || area.north_west.lat < self.south_east.lat {
            return false;
        }

        true
    }
}

mod tests {
    use crate::{Bounds, Coordinate};

    #[test]
    fn contains() {
        let bounds = Bounds {
            north_west: Coordinate { lon: -1., lat: 1. },
            south_east: Coordinate { lon: 1., lat: -1. },
        };

        assert!(!bounds.contains(&Coordinate { lat: -2., lon: -2. }));
        assert!(!bounds.contains(&Coordinate { lat: 2., lon: -2. }));
        assert!(!bounds.contains(&Coordinate { lat: -2., lon: 2. }));
        assert!(!bounds.contains(&Coordinate { lat: 2., lon: 2. }));

        assert!(bounds.contains(&Coordinate { lat: 0., lon: 0. }));
    }

    #[test]
    fn intersects() {
        let bounds = Bounds {
            north_west: Coordinate { lon: -1., lat: 1. },
            south_east: Coordinate { lon: 1., lat: -1. },
        };

        assert!(bounds.intersects(&Bounds {
            north_west: Coordinate { lat: 0., lon: 0. },
            south_east: Coordinate { lat: 0., lon: 0. },
        }));

        assert!(!bounds.intersects(&Bounds {
            north_west: Coordinate { lat: 3., lon: 3. },
            south_east: Coordinate { lat: 4., lon: 4. },
        }));

        assert!(!bounds.intersects(&Bounds {
            north_west: Coordinate { lat: 3., lon: -3. },
            south_east: Coordinate { lat: 4., lon: -4. },
        }));

        assert!(!bounds.intersects(&Bounds {
            north_west: Coordinate { lat: -3., lon: -3. },
            south_east: Coordinate { lat: -4., lon: -4. },
        }));

        assert!(!bounds.intersects(&Bounds {
            north_west: Coordinate { lat: -3., lon: 3. },
            south_east: Coordinate { lat: -4., lon: 4. },
        }));

        assert!(bounds.intersects(&Bounds {
            north_west: Coordinate { lat: 0., lon: 0. },
            south_east: Coordinate { lat: -4., lon: 4. },
        }));

        assert!(bounds.intersects(&Bounds {
            north_west: Coordinate { lat: 0., lon: -4. },
            south_east: Coordinate { lat: -4., lon: 0. },
        }));

        assert!(bounds.intersects(&Bounds {
            north_west: Coordinate { lat: 4., lon: -4. },
            south_east: Coordinate { lat: 0., lon: 0. },
        }));

        assert!(bounds.intersects(&Bounds {
            north_west: Coordinate { lat: 4., lon: 0. },
            south_east: Coordinate { lat: 0., lon: 4. },
        }));

        assert!(bounds.intersects(&Bounds {
            north_west: Coordinate { lat: -1., lon: -1. },
            south_east: Coordinate {
                lat: 0.5,
                lon: -0.5,
            },
        }));
    }
}

impl<T> GeoTree<T>
where
    T: Copy + Clone,
{
    pub fn create<D>(input_data: Vec<Vec<T>>, bounds: Bounds) -> Self
    where
        D: Dataset<T>,
    {
        // let leaf_nodes = extract_blocks(&input_data, D::TILE_SIZE, D::TILE_SIZE);

        let lat_step =
            (bounds.north_west.lat - bounds.south_east.lat).abs() / input_data.len() as f32; // Check this
        let lon_step =
            (bounds.north_west.lon - bounds.south_east.lon).abs() / input_data[0].len() as f32;

        let (root, depth) = create_recursive::<T, D>(&input_data, bounds, lat_step, lon_step, 0);

        GeoTree { root, depth }
    }

    pub fn get_tile(&self, point: Coordinate, level: u32) -> Option<&Tile<T>> {
        let mut current_level = 0;
        let mut target = &self.root;

        while current_level <= level {
            for node in target.children.iter().flatten() {
                if node.bounds.contains(&point) {
                    target = node;
                    current_level += 1;

                    if current_level == level {
                        return Some(&target.tile);
                    }
                }
            }
        }

        None
    }

    pub fn get_tiles(&self, area: Bounds, level: u32) -> Vec<TileRef<'_, T>> {
        fn f<T>(
            level: u32,
            current_level: u32,
            node: &TileNode<T>,
            area: Bounds,
        ) -> Option<Vec<TileRef<'_, T>>> {
            dbg!(current_level);

            if dbg!(node.bounds.intersects(&area)) {
                if current_level == level {
                    dbg!(node.bounds, current_level);
                    return Some(vec![node.into()]);
                }

                return Some(
                    node.children
                        .iter()
                        .flatten()
                        .flat_map(|c| f(level, current_level + 1, c, area))
                        .flatten()
                        .collect(),
                );
            }

            None
        }

        f(level, 0, &self.root, area).unwrap()
    }
}

#[derive(Serialize)]
pub struct TileRef<'a, T> {
    pub tile: &'a Tile<T>,
    pub bounds: Bounds,
}

impl<'a, T> Into<TileRef<'a, T>> for &'a TileNode<T> {
    fn into(self) -> TileRef<'a, T> {
        TileRef {
            tile: &self.tile,
            bounds: self.bounds,
        }
    }
}

pub trait Dataset<T> {
    fn aggregate(data: Vec<T>) -> T;
    fn convolute(data: Vec<Vec<T>>, level: usize) -> Vec<Vec<T>>;

    const STRIDE: usize;
    const TILE_SIZE: usize;
    const NUMBER_OF_CHILDREN: (usize, usize);
}

pub struct EarthTextures;

pub type Pixel = [u8; 3];

impl Dataset<Pixel> for EarthTextures {
    fn aggregate(data: Vec<Pixel>) -> Pixel {
        Default::default()
    }

    fn convolute(data: Vec<Vec<Pixel>>, level: usize) -> Vec<Vec<Pixel>> {
        let h = Self::NUMBER_OF_CHILDREN.1.pow(level as u32);
        let w = Self::NUMBER_OF_CHILDREN.0.pow(level as u32);

        extract_blocks(&data, h, w)
            .into_iter()
            .map(|row| {
                row.into_iter()
                    .map(|tile| {
                        let mut kernel_value = [0_usize; 4];
                        let mut n_values = 0;
                        tile.iter().flatten().for_each(|pixel| {
                            n_values += 1;
                            kernel_value[0] += pixel[0] as usize;
                            kernel_value[1] += pixel[1] as usize;
                            kernel_value[2] += pixel[2] as usize;
                        });

                        kernel_value[0] /= n_values;
                        kernel_value[1] /= n_values;
                        kernel_value[2] /= n_values;
                        [
                            kernel_value[0] as u8,
                            kernel_value[1] as u8,
                            kernel_value[2] as u8,
                        ]
                    })
                    .collect()
            })
            .collect()
    }

    const STRIDE: usize = 0;
    const NUMBER_OF_CHILDREN: (usize, usize) = (2, 2);
    const TILE_SIZE: usize = 512;
}

pub fn stitch_tiles<T>(tiles: Vec<Vec<TileNode<T>>>) -> Vec<Vec<T>>
where
    T: Copy + Default,
{
    let mut buffer = vec![];

    for row in tiles {
        let mut tile_main = row[0].tile.clone();
        for tile_ext in 1..row.len() {
            tile_main
                .iter_mut()
                .zip(row[tile_ext].tile.iter())
                .into_iter()
                .for_each(|(row1, row2)| {
                    row1.extend(row2);
                });
        }

        buffer.append(&mut tile_main);
    }

    buffer
}
fn extract_blocks<T: Clone>(
    matrix: &[Vec<T>],
    block_height: usize,
    block_width: usize,
) -> Vec<Vec<Vec<Vec<T>>>> {
    if block_width <= 0 || block_height <= 0 {
        // panic!("{}, {}", block_width, block_height);
        return vec![];
    }

    matrix
        .chunks(block_height)
        .map(|row_chunk| {
            (0..row_chunk[0].len())
                .step_by(block_width)
                .map(|col_start| {
                    row_chunk
                        .iter()
                        .map(|row| {
                            row.iter()
                                .skip(col_start)
                                .take(block_width)
                                .cloned()
                                .collect::<Vec<T>>()
                        })
                        .collect::<Vec<Vec<T>>>()
                })
                .collect::<Vec<Vec<Vec<T>>>>()
        })
        .collect()
}

fn split_into_n_m_with_min_size<T: Clone, D>(
    data: &[Vec<T>],
    n: usize,
    m: usize,
) -> Vec<Vec<Vec<Vec<T>>>>
where
    D: Dataset<T>,
{
    let width = (data[0].len() + 1) / m;
    let height = (data.len() + 1) / n;

    if width < D::TILE_SIZE {
        return vec![];
    }

    extract_blocks(data, height, width)
}

pub fn create_recursive<T: Clone, D>(
    input: &[Vec<T>],
    bounds: Bounds,
    lat_step: f32,
    lon_step: f32,
    level: usize,
) -> (TileNode<T>, usize)
where
    D: Dataset<T>,
{
    let mut depth = level;
    let children = split_into_n_m_with_min_size::<T, D>(
        input,
        D::NUMBER_OF_CHILDREN.1,
        D::NUMBER_OF_CHILDREN.0,
    )
    .into_iter()
    .enumerate()
    .map(|(y, row): (usize, Vec<Vec<Vec<T>>>)| {
        row.into_iter()
            .enumerate()
            .map(|(x, new_tile): (usize, Vec<Vec<T>>)| {
                let new_bounds = Bounds {
                    north_west: Coordinate {
                        lat: bounds.north_west.lat - lat_step * (y as f32),
                        lon: bounds.north_west.lon + lon_step * (y as f32),
                    },
                    south_east: Coordinate {
                        lat: bounds.north_west.lat - lat_step * (y + 1) as f32,
                        lon: bounds.north_west.lon + lon_step * (x + 1) as f32,
                    },
                };
                let (child, d) =
                    create_recursive::<T, D>(&new_tile, new_bounds, lat_step, lon_step, level + 1);

                depth = depth.max(d);
                child
            })
            .collect()
    })
    .collect();
    dbg!(level);
    (
        TileNode {
            bounds,
            aggregate: D::aggregate(input.into_iter().flatten().cloned().collect()),
            tile: D::convolute(input.to_vec(), depth + 1 - level),
            children,
        },
        depth,
    )
}
