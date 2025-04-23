#[derive(Copy, Clone, Debug)]
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

impl<T> GeoTree<T> {
    pub fn get_tile(&self, depth: usize, point: &Coordinate) -> Option<&Tile<T>> {
        let mut target = &self.root;
        let mut current_depth = self.depth;

        if current_depth == depth {
            return Some(&self.root.tile);
        }

        println!("Total depth: {}", self.depth);

        for node in target.children.iter().flatten() {
            dbg!(node.bounds);
            if node.bounds.contains(&point) {
                target = node;
                current_depth -= 1;

                println!("Going down to depth: {}", current_depth);
                if current_depth == depth {
                    return Some(&target.tile);
                }
            }
        }

        None
    }
}

#[derive(Copy, Clone, Debug)]
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
}

mod tests {
    use crate::{Bounds, Coordinate};

    #[test]
    fn test() {
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

        let root = create_recursive::<T, D>(&input_data, bounds, lat_step, lon_step, 0);

        let mut depth = 0;
        find_depth(&root, &mut depth);
        GeoTree {
            root,
            depth: depth as usize,
        }
    }
}

pub trait Dataset<T> {
    fn aggregate(data: Vec<T>) -> T;
    fn convolute(data: Vec<Vec<T>>) -> Vec<Vec<T>>;

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

    fn convolute(data: Vec<Vec<Pixel>>) -> Vec<Vec<Pixel>> {
        let h = data.len() / Self::TILE_SIZE;
        let w = data[0].len() / Self::TILE_SIZE;

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
    const NUMBER_OF_CHILDREN: (usize, usize) = (4, 2);
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

    // for y in 0..tiles.len() {
    //     for x in 0..tiles[y].len() {
    //         let tile = &tiles[y][x].tile; // it is a trap

    //         println!("(buffer) y: {}", buffer.len());
    //         println!("(buffer) x: {}", buffer[0].len());
    //         println!("(tile) y: {}", tile.len());
    //         println!("(tile) x: {}", tile[0].len());
    //         println!("(input) y: {}", tiles.len());
    //         println!("(input) x: {}", tiles[0].len());

    //         tile.into_iter().enumerate().for_each(|t_y, row| {
    //             row.into_iter().enumerate().for_each(|t_x, value| {
    //                 let x = t_x + x * tile_size;
    //                 kkk
    //             })
    //         });
    //         for j in 0..tile.len() {
    //             for i in 0..tile[j].len() {
    //                 let x_i = x * tile_size + i;
    //                 let y_i = y * tile_size + j;
    //                 if x_i >= tiles[0].len() * tile_size {
    //                     dbg!(tile_size);
    //                     dbg!(i);
    //                     dbg!(tile.len());
    //                     dbg!(tile[j].len());

    //                     dbg!(x);
    //                     dbg!(y);
    //                     dbg!(x_i);
    //                     dbg!(y_i);
    //                     dbg!(buffer.len());
    //                     dbg!(buffer[0].len());
    //                 }

    //                 buffer[y_i][x_i] = tile[j][i];
    //             }
    //         }
    //     }
    // }

    buffer
}
fn extract_blocks<T: Clone>(
    matrix: &[Vec<T>],
    block_height: usize,
    block_width: usize,
) -> Vec<Vec<Vec<Vec<T>>>> {
    if block_width <= 0 || block_height <= 0 {
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

    extract_blocks(data, height, width)
}

pub fn create_recursive<T: Clone, D>(
    input: &[Vec<T>],
    bounds: Bounds,
    lat_step: f32,
    lon_step: f32,
    level: u32,
) -> TileNode<T>
where
    D: Dataset<T>,
{
    dbg!(level);
    TileNode {
        bounds,
        aggregate: D::aggregate(input.into_iter().flatten().cloned().collect()),
        tile: D::convolute(input.to_vec()),
        children: split_into_n_m_with_min_size::<T, D>(
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
                    create_recursive::<T, D>(&new_tile, new_bounds, lat_step, lon_step, level + 1)
                })
                .collect()
        })
        .collect(),
    }
}

fn find_depth<T>(root: &TileNode<T>, depth: &mut u32) {
    if let Some(child) = root.children.first().and_then(|x| x.first()) {
        *depth += 1;
        find_depth(child, depth);
    }
}
