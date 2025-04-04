use axum::Router;
use std::{
    net::SocketAddr,
    ops::{Index, Range},
};
use tower_http::{services::ServeDir, trace::TraceLayer};

#[tokio::main]
async fn main() {
    let addr = SocketAddr::from(([127, 0, 0, 1], 8000));

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    let router = Router::new().fallback_service(ServeDir::new(env!("ASSETS_DIR")));

    axum::serve(listener, router.layer(TraceLayer::new_for_http()))
        .await
        .unwrap();
}

// enum NodeType<T> {
// Leaf,
// Parent(Vec<Node<T>>),
// }

struct Coordinates {
    lat: f32,
    lon: f32,
}

// struct LeafNode<T> {
//     bounds: Bounds,
//     value: T,
// }

struct Node<T> {
    children: Option<Vec<Node<T>>>,
    value: T,
    bounds: Bounds,
}

// impl<T> LeafNode<T> {}

// struct ParentNode<T> {
//     value: T,
//     bounds: Bounds,
//     children: Vec<Node<T>>,
// }

impl<T> Node<T> {
    fn new_parent_from_children<F>(nodes: Vec<Node<T>>) -> Self
    where
        F: Dataset<T>,
    {
        let mut bounds = Bounds {
            north_west: Coordinates {
                lat: f32::MIN,
                lon: f32::MAX,
            },
            south_east: Coordinates {
                lat: f32::MAX,
                lon: f32::MIN,
            },
        };

        for node in &nodes {
            bounds.north_west.lat = bounds.north_west.lat.max(node.bounds.north_west.lat);
            bounds.north_west.lon = bounds.north_west.lon.min(node.bounds.north_west.lon);
            bounds.south_east.lat = bounds.south_east.lat.min(node.bounds.south_east.lat);
            bounds.south_east.lon = bounds.south_east.lon.max(node.bounds.south_east.lon);
        }

        Self {
            value: F::aggregate(nodes.iter().map(|x| &x.value).collect()),
            bounds,
            children: Some(nodes),
        }
    }
}

struct GeoTree<T> {
    root: Node<T>,
}

struct Bounds {
    north_west: Coordinates,
    south_east: Coordinates,
}

impl<T> GeoTree<T> {
    fn create<D>(input_data: Vec<Vec<T>>, bounds: Bounds) -> Self
    where
        D: Dataset<T>,
    {
        // assert_eq!(T::MASK_DIM. % 4, 0);
        //
        //
        //
        //
        //
        let lat_step =
            (bounds.north_west.lat - bounds.south_east.lat).abs() / input_data.len() as f32; // Check this
        let lon_step =
            (bounds.north_west.lon - bounds.south_east.lon).abs() / input_data[0].len() as f32;

        let mut leaf_nodes: Vec<Vec<Node<T>>> = input_data
            .into_iter()
            .enumerate()
            .map(|(y, y_val)| {
                y_val
                    .into_iter()
                    .enumerate()
                    .map(|(x, value)| {
                        let bounds = Bounds {
                            north_west: Coordinates {
                                lat: bounds.north_west.lat - lat_step * (y as f32),
                                lon: bounds.north_west.lon + lon_step * (x as f32),
                            },
                            south_east: Coordinates {
                                lat: bounds.north_west.lat - lat_step * ((y + 1) as f32),
                                lon: bounds.north_west.lon + lon_step * ((x + 1) as f32),
                            },
                        };

                        Node {
                            children: None,
                            value,
                            bounds,
                        }
                    })
                    .collect::<Vec<Node<T>>>()
            })
            .collect();

        let mut levels = vec![leaf_nodes];

        while levels.last().unwrap().len() > 1 {
            let parents: Vec<Vec<Node<T>>> = levels.last();
            for y in (0..parents.len()).step_by(D::MASK_DIM.1) {
                for x in (0..parents[0].len()).step_by(D::MASK_DIM.0) {
                    for dy in 0..D::MASK_DIM.1 {
                        for dx in 0..D::MASK_DIM.0 {
                            let x = dx + x;
                            let y = dy + y;
                        }
                    }
                }
            }
        }
        todo!()

        // let mut node = Vec::new();

        // for y in 0..leaf_nodes.len() / D::MASK_DIM.1 {
        //     let y = y * D::MASK_DIM.1;

        //     let mut y_slice = leaf_nodes.drain(y..y + D::MASK_DIM.1).collect::<Vec<_>>();

        //     for x in 0..y_slice[0].len() / D::MASK_DIM.0 {
        //         let x = x * D::MASK_DIM.0;

        //         let mut children = vec![];

        //         for y in 0..D::MASK_DIM.1 {
        //             let v = y_slice[y].drain(x..x + D::MASK_DIM.0);
        //             children.extend(v);
        //         }

        //         let parent = Node::new_parent_from_children::<D>(children);
        //         node.push(parent);
        //     }
        // }
        //
        //

        // let parents = convolve::<_, D>(leaf_nodes);

        // let f = |nodes: Vec<Node<T>>| {
        //     if nodes.len() == 1 {
        //         return nodes;
        //     }

        //     Node::new_parent_from_children(nodes)
        // };

        // let g = |nodes: Vec<Node<T>>| -> Node<T> {
        //     let mut children = Vec::new();

        //     for y in 0..10 {
        //         for x in 0..10 {
        //             let child = g(nodes[x]);
        //             children.push(child);
        //         }
        //     }

        //     Node::new_parent_from_children(children)
        // };

        // loop {
        //     let parents = convolve::<_, D>(nodes);

        //     if parents.len() == 1 {
        //         break;
        //     }
        // }

        // todo!()
    }
}

fn convolve<T, D>(
    mut data: Vec<Vec<Node<T>>>,
    // kernel: (usize, usize),
    // stride: usize,
) -> Vec<Node<T>>
where
    D: Dataset<T>,
{
    let mut parents = Vec::new();

    for y in 0..data.len() / D::MASK_DIM.1 {
        let y = y * D::MASK_DIM.1;

        let mut y_slice = data.drain(y..y + D::MASK_DIM.1).collect::<Vec<_>>();

        for x in 0..y_slice[0].len() / D::MASK_DIM.0 {
            let x = x * D::MASK_DIM.0;

            let mut children = vec![];

            for y in 0..D::MASK_DIM.1 {
                let v = y_slice[y].drain(x..x + D::MASK_DIM.0);
                children.extend(v);
            }

            let parent = Node::new_parent_from_children::<D>(children);
            parents.push(parent);
        }
    }

    parents
}

// impl Dataset<f32> for f32 {
//     fn aggregate(data: Vec<Self>) -> Self {
//         data.iter().sum()
//     }

//     const MASK_DIM: (usize, usize) = todo!();
// }

trait SliceExt<T> {
    fn safe_slice(&self, range: Range<usize>) -> Option<&[T]>;
}

impl<T> SliceExt<T> for Vec<T> {
    fn safe_slice(&self, range: Range<usize>) -> Option<&[T]> {
        let start = range.start;
        let mut end = range.end;

        loop {
            if let Some(slice) = self.get(start..end) {
                return Some(slice);
            }

            end -= 1;

            if start == end {
                return None;
            }
        }
    }
}

trait Dataset<T> {
    fn aggregate(data: Vec<&T>) -> T;

    const MASK_DIM: (usize, usize);
    const STRIDE: usize;
}
