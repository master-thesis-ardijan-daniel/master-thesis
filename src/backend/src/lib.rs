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
            tile: F::convolute(nodes.clone()),
            children: nodes,
        }
    }
}

pub struct GeoTree<T> {
    pub root: TileNode<T>,
    depth: usize,
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
        let leaf_nodes = split_into_tiles::<D, _>(input_data);

        let lat_step =
            (bounds.north_west.lat - bounds.south_east.lat).abs() / leaf_nodes.len() as f32; // Check this
        let lon_step =
            (bounds.north_west.lon - bounds.south_east.lon).abs() / leaf_nodes[0].len() as f32;

        let leaf_nodes = leaf_nodes
            .into_iter()
            .enumerate()
            .map(|(y, row)| {
                row.into_iter()
                    .enumerate()
                    .map(|(x, tile)| {
                        let bounds = Bounds {
                            north_west: Coordinate {
                                lat: bounds.north_west.lat - lat_step * (y as f32),
                                lon: bounds.north_west.lon + lon_step * (y as f32),
                            },
                            south_east: Coordinate {
                                lat: bounds.north_west.lat - lat_step * (y + 1) as f32,
                                lon: bounds.north_west.lon + lon_step * (x + 1) as f32,
                            },
                        };

                        TileNode {
                            children: Vec::new(),
                            aggregate: D::aggregate(tile.clone().into_iter().flatten().collect()),
                            bounds,
                            tile,
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        let mut previous_layer = leaf_nodes;
        let mut current_layer = Vec::new();
        let mut depth = 0;

        loop {
            depth += 1;
            for y in (0..previous_layer.len()).step_by(D::TILE_SIZE) {
                let mut row = Vec::new();

                for x in (0..previous_layer[0].len()).step_by(D::TILE_SIZE) {
                    let mut nodes = Vec::new();
                    'y: for dy in 0..D::TILE_SIZE {
                        let mut node_row = Vec::new();

                        'x: for dx in 0..D::TILE_SIZE {
                            let x = dx + x;
                            let y = dy + y;

                            if x >= previous_layer[0].len() {
                                break 'x;
                            }

                            if y >= previous_layer.len() {
                                break 'y;
                            }

                            let node = previous_layer[y][x].clone();
                            node_row.push(node);
                        }
                        nodes.push(node_row);
                    }

                    let node = TileNode::new_parent_from_children::<D>(nodes);

                    row.push(node);
                }

                current_layer.push(row);
            }

            break;
            if current_layer.len() <= 1 {
                break;
            }

            previous_layer = current_layer;
            current_layer = Vec::new();
        }

        let root = current_layer.into_iter().flatten().next().unwrap();

        GeoTree { root, depth }
    }
}

pub trait Dataset<T> {
    fn aggregate(data: Vec<T>) -> T;
    fn convolute(data: Vec<Vec<TileNode<T>>>) -> Vec<Vec<T>>;

    const MASK_DIM: (usize, usize);
    const STRIDE: usize;
    const TILE_SIZE: usize;
}

fn split_into_tiles<D, T>(data: Vec<Vec<T>>) -> Vec<Vec<Tile<T>>>
where
    D: Dataset<T>,
    T: Copy,
{
    let mut out = Vec::new();

    for y in (0..data.len()).step_by(D::TILE_SIZE) {
        let mut row = Vec::new();

        for x in (0..data[0].len()).step_by(D::TILE_SIZE) {
            let mut tile = Vec::new();
            'y: for dy in 0..D::TILE_SIZE {
                let mut tile_row = Vec::new();

                'x: for dx in 0..D::TILE_SIZE {
                    let x = dx + x;
                    let y = dy + y;

                    if x >= data[0].len() {
                        break 'x;
                    }

                    if y >= data.len() {
                        break 'y;
                    }

                    let value = data[y][x];
                    tile_row.push(value);
                }
                tile.push(tile_row);
            }

            row.push(tile);
        }

        out.push(row);
    }

    out
}

pub struct EarthTextures;

pub type Pixel = [u8; 3];

impl Dataset<Pixel> for EarthTextures {
    fn aggregate(data: Vec<Pixel>) -> Pixel {
        Default::default()
    }

    fn convolute(data: Vec<Vec<TileNode<Pixel>>>) -> Vec<Vec<Pixel>> {
        let full_image = stitch_tiles(data);

        let mut out = Vec::new();

        for y in (0..full_image.len()).step_by(Self::MASK_DIM.1) {
            let mut row = Vec::new();
            for x in (0..full_image[0].len()).step_by(Self::MASK_DIM.0) {
                let mut kernel_value = [0_usize; 4];

                'y: for dy in 0..Self::MASK_DIM.1 {
                    'x: for dx in 0..Self::MASK_DIM.0 {
                        let x = dx + x;
                        let y = dy + y;

                        if y >= full_image.len() {
                            break 'y;
                        }

                        if x >= full_image[y].len() {
                            break 'x;
                        }

                        let value = full_image[y][x];

                        kernel_value[0] += value[0] as usize;
                        kernel_value[1] += value[1] as usize;
                        kernel_value[2] += value[2] as usize;
                    }
                }

                kernel_value[0] /= Self::MASK_DIM.1 * Self::MASK_DIM.0;
                kernel_value[1] /= Self::MASK_DIM.1 * Self::MASK_DIM.0;
                kernel_value[2] /= Self::MASK_DIM.1 * Self::MASK_DIM.0;

                let kernel_value = [
                    kernel_value[0] as u8,
                    kernel_value[1] as u8,
                    kernel_value[2] as u8,
                ];

                row.push(kernel_value);
            }

            out.push(row);
        }

        out
    }

    const MASK_DIM: (usize, usize) = (2, 2);

    const STRIDE: usize = 0;

    const TILE_SIZE: usize = 512;
}

pub fn stitch_tiles<T>(tiles: Vec<Vec<TileNode<T>>>) -> Vec<Vec<T>>
where
    T: Copy + Default,
{
    let tile_size = tiles[0][0].tile.len();

    let mut buffer = vec![vec![T::default(); tiles[0].len() * tile_size]; tiles.len() * tile_size];

    for y in 0..tiles.len() {
        for x in 0..tiles[0].len() {
            let tile = &tiles[y][x].tile;

            for j in 0..tile.len() {
                for i in 0..tile[j].len() {
                    let x = x * tile_size + i;
                    let y = y * tile_size + j;

                    if x >= tiles[0].len() * tile_size {
                        println!("(buffer) y: {}", buffer.len());
                        println!("(buffer) x: {}", buffer[0].len());
                        println!("(tile) y: {}", tile.len());
                        println!("(tile) x: {}", tile[0].len());
                        println!("(input) y: {}", tiles.len());
                        println!("(input) x: {}", tiles[0].len());
                    }

                    buffer[y][x] = tile[j][i];
                }
            }
        }
    }

    buffer
}
