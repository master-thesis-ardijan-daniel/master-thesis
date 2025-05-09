use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use bytemuck::{Pod, Zeroable};
use geo::{Coord, CoordNum, Intersects};
use itertools::Itertools;

use crate::{Bounds, Dataset};

pub trait Deserialize<'de> {
    fn deserialize(bytes: &'de [u8]) -> (usize, Self);
}

// #[derive(Clone, Copy)]
pub struct Reader<'a> {
    pub inner: &'a [u8],
    pub position: AtomicUsize,
}

fn test() {
    // let mut reader = Reader {
    //     inner: Arc::new(unsafe { memmap2::Mmap::map(&std::fs::File::open("").unwrap()).unwrap() }),
    //     position: 0,
    // };

    // let root = reader.read::<TileNode<f32>>();
    // let child = reader.load(&root.children[0][0])
}

impl<'a> Reader<'a> {
    pub fn read<T>(&'a self) -> T
    where
        T: Deserialize<'a>,
    {
        let position = self.position.load(Ordering::Relaxed);

        let (read, out) = T::deserialize(&self.inner[position..]);

        self.position.fetch_add(read, Ordering::Relaxed);

        out
    }

    pub fn load<T>(&'a self, pointer: &Pointer<T>) -> TileNode<'a, T>
    where
        T: Pod,
    {
        dbg!(pointer.position);
        let position = self.position.fetch_add(pointer.position, Ordering::Relaxed);

        let (read, out) = Deserialize::deserialize(&self.inner[position + pointer.position..]);

        self.position
            .store(pointer.position + read, Ordering::Relaxed);

        out
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Pointer<T> {
    pub position: usize,
    _type: std::marker::PhantomData<T>,
}

impl<T> Default for Pointer<T> {
    fn default() -> Self {
        Self {
            position: 0,
            _type: Default::default(),
        }
    }
}

pub struct TileNode<'a, T> {
    pub bounds: Bounds,
    pub children: Vec<&'a [Pointer<T>]>,
}

pub struct TileData<'a, T> {
    pub aggregate: Option<&'a T>,
    pub tile: Option<Vec<&'a [T]>>,
}

impl<'a, T> Deserialize<'a> for TileData<'a, T>
where
    T: Pod,
{
    fn deserialize(bytes: &'a [u8]) -> (usize, Self) {
        let mut cur = 0;

        let (read, aggregate) = Deserialize::deserialize(&bytes[cur..]);
        cur += read;

        let (read, tile) = Deserialize::deserialize(&bytes[cur..]);
        cur += read;

        (cur, Self { aggregate, tile })
    }
}

impl<'a, T> Deserialize<'a> for TileNode<'a, T>
where
    T: Pod,
{
    fn deserialize(bytes: &'a [u8]) -> (usize, Self) {
        let mut cur = 0;

        let (read, bounds) = Deserialize::deserialize(&bytes[cur..]);
        dbg!(cur, read);
        cur += read;

        let (read, children) = Deserialize::deserialize(&bytes[cur..]);
        dbg!(cur, read);
        cur += read;

        (cur, Self { bounds, children })
    }
}

struct TileRef<'a, T> {
    _marker: std::marker::PhantomData<&'a T>,
}

struct GeoTree<'a, D>
where
    D: Dataset,
{
    root: TileNode<'a, D::Type>,
    reader: Reader<'a>,
}

struct TileIterator<'a, T> {
    query: Bounds,
    level: usize,
    reader: Reader<'a>,
    queue: Vec<&'a Pointer<T>>,
}

impl<'a, D> GeoTree<'a, D>
where
    D: Dataset,
{
    fn get_tiles(&'a mut self, area: Bounds, level: u32) -> Vec<TileData<'a, D::Type>>
    where
        D::Type: Pod,
    {
        fn inner<'a, T>(
            level: u32,
            current_level: u32,
            pointer: &Pointer<T>,
            area: Bounds,
            reader: &'a Reader<'a>,
        ) -> Option<Vec<TileData<'a, T>>>
        where
            T: Pod,
        {
            let node = reader.load(pointer);

            if node.bounds.intersects(&area) {
                if current_level == level {
                    let tile = reader.read::<TileData<T>>();
                    return Some(vec![tile]);
                }

                Some(
                    node.children
                        .iter()
                        .copied()
                        .flatten()
                        .flat_map(|child| inner(level, current_level + 1, &child, area, reader))
                        .flatten()
                        .collect(),
                )
            } else {
                None
            }
        }

        inner(level, 0, &Pointer::default(), area, &mut self.reader).unwrap()
    }
}

impl<'a, T: Pod> Deserialize<'a> for &'a T {
    fn deserialize(bytes: &'a [u8]) -> (usize, Self) {
        let mut cur = 0;

        let read = std::mem::size_of::<T>();
        let out = bytemuck::from_bytes::<T>(&bytes[cur..cur + read]);
        cur += read;

        (cur, out)
    }
}

impl<T> Deserialize<'_> for Coord<T>
where
    T: CoordNum + Pod,
{
    fn deserialize(bytes: &[u8]) -> (usize, Self) {
        let mut cur = 0;

        let read = std::mem::size_of::<T>();

        let x = *bytemuck::from_bytes::<T>(&bytes[cur..cur + read]);
        cur += read;

        let read = std::mem::size_of::<T>();
        let y = *bytemuck::from_bytes::<T>(&bytes[cur..cur + read]);
        cur += read;

        (cur, Self { x, y })
    }
}

impl<'a> Deserialize<'a> for Bounds {
    fn deserialize(bytes: &[u8]) -> (usize, Self) {
        let mut cur = 0;

        let (read, min) = Coord::deserialize(&bytes[cur..]);
        cur += read;

        let (read, max) = Coord::deserialize(&bytes[cur..]);
        cur += read;

        (cur, Self::new(min, max))
    }
}

impl<'a, T> Deserialize<'a> for &'a [T]
where
    T: Pod,
{
    fn deserialize(bytes: &'a [u8]) -> (usize, Self) {
        let mut cur = 0;

        let read = std::mem::size_of::<usize>();
        let len = *bytemuck::from_bytes::<usize>(&bytes[cur..cur + read]);
        dbg!(cur, read, len);
        cur += read;

        let read = len * std::mem::size_of::<T>();
        dbg!(read);
        let array = bytemuck::cast_slice(&bytes[cur..cur + read]);
        cur += read;

        (cur, array)
    }
}

impl<'a, T> Deserialize<'a> for Vec<&'a [T]>
where
    T: Pod,
{
    fn deserialize(bytes: &'a [u8]) -> (usize, Self) {
        let mut cur = 0;

        println!("start");
        let read = std::mem::size_of::<usize>();
        println!("{:#?}, {}", &bytes[cur..cur + read], read);
        let height = bytemuck::from_bytes::<usize>(&bytes[cur..cur + read]);
        println!("Read height");
        cur += read;
        dbg!(height);

        let mut out = Vec::with_capacity(*height);

        for i in 0..*height {
            println!("iteration {i}");

            let (pos, array) = <&[T] as Deserialize>::deserialize(&bytes[cur..]);

            out.push(array);

            cur += pos;
        }

        (cur, out)
    }
}

impl<'a, T> Deserialize<'a> for Option<T>
where
    T: Deserialize<'a>,
{
    fn deserialize(bytes: &'a [u8]) -> (usize, Self) {
        let mut cur = 0;

        let read = std::mem::size_of::<usize>();
        let option = bytemuck::from_bytes::<usize>(&bytes[cur..cur + read]);
        cur += read;

        dbg!(cur, option);

        if *option != 1 {
            println!("was empty");
            return (cur, None);
        }

        let (read, value) = Deserialize::deserialize(&bytes[cur..]);
        cur += read;

        (cur, Some(value))
    }
}
