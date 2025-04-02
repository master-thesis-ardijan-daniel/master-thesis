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

enum Node<T> {
    Leaf(LeafNode<T>),
    Parent(ParentNode<T>),
}

struct Coordinates {
    lat: f32,
    lon: f32,
}

struct LeafNode<T> {
    bounds: Bounds,
    value: T,
}

struct ParentNode<T> {
    bounds: Bounds,
    children: Vec<Node<T>>,
}

struct GeoTree<T> {
    root: Node<T>,
}

struct Bounds {
    north_west: Coordinates,
    south_east: Coordinates,
}

impl<T> GeoTree<T>
where
    T: Dataset<T>,
{
    fn create(input_data: Vec<Vec<T>>, bounds: Bounds) -> Self {
        let root = ParentNode {
            bounds,
            children: Vec::<Node<T>>::new(),
        };

        // assert_eq!(T::MASK_DIM. % 4, 0);
        //
        //

        let mut s = vec![];

        for y in 0..input_data.len() / T::MASK_DIM.1 {
            let y = y * T::MASK_DIM.1;
            let y_mask = input_data.safe_slice(y..y + T::MASK_DIM.1);
            for x in 0..input_data[y].len() / T::MASK_DIM.0 {}
        }

        todo!()
    }
}

// impl Dataset<f32> for f32 {
//     fn aggregate(data: Vec<Self>) -> Self {
//         data.iter().sum()
//     }

//     const MASK_DIM: (usize, usize) = todo!();
// }

trait SliceExt<T> {
    fn safe_slice(&self, range: Range<usize>) -> &[T];
}

impl<T> SliceExt<T> for Vec<T> {
    fn safe_slice(&self, range: Range<usize>) -> &[T] {
        let start = range.start;
        let mut end = range.end;

        loop {
            if let Some(slice) = self.get(start..end) {
                return slice;
            }

            end -= 1;

            if start == end {
                return &[];
            }
        }
    }
}

trait Dataset<T> {
    fn aggregate(data: Vec<T>) -> T;

    /// Value must be a power of 4.
    const MASK_DIM: (usize, usize);
    const STRIDE: usize;
}
