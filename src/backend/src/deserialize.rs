use bytemuck::Pod;
use geo::{Coord, CoordNum};

pub trait Deserialize<'de> {
    fn deserialize(bytes: &'de [u8]) -> (usize, Self);
}

impl<T> Deserialize<'_> for Coord<T>
where
    for<'a> T: CoordNum + Deserialize<'a>,
{
    fn deserialize(bytes: &[u8]) -> (usize, Self) {
        todo!()
    }
}

impl<'a, T> Deserialize<'a> for &'a [T]
where
    T: Pod,
{
    fn deserialize(bytes: &'a [u8]) -> (usize, Self) {
        let mut cur = 0;

        let read = std::mem::size_of::<usize>();
        let len = *bytemuck::from_bytes::<usize>(&bytes[cur..read]);
        cur += read;

        let read = cur + len * std::mem::size_of::<T>();
        let array = bytemuck::cast_slice(&bytes[cur..read]);
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

        let read = std::mem::size_of::<usize>();
        let height = *bytemuck::from_bytes::<usize>(&bytes[cur..read]);
        cur += read;

        let mut out = Vec::with_capacity(height);

        for _ in 0..height {
            let (pos, array) = <&[T] as Deserialize>::deserialize(&bytes[cur..]);

            out.push(array);

            cur = pos;
        }

        (cur, out)
    }
}
