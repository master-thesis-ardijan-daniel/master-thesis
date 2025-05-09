use std::{
    borrow::Cow,
    collections::{HashMap, VecDeque},
    io::{Result, Write},
};

use bytemuck::{write_zeroes, Pod};
use common::Bounds;
use geo::{BoundingRect, Coord, CoordNum};
use tokio::io::AsyncWriteExt;

use crate::TileNode;

pub trait Serialize {
    fn serialize<W>(&self, writer: &mut W) -> Result<usize>
    where
        W: Write;
}

impl<T> Serialize for TileNode<T>
where
    T: Pod,
{
    fn serialize<W>(&self, writer: &mut W) -> Result<usize>
    where
        W: Write,
    {
        let mut queue = VecDeque::new();
        queue.push_back(self);

        let mut written = HashMap::<usize, usize>::new();
        let mut sink = std::io::sink();

        while let Some(node) = queue.pop_front() {
            let wrote = {
                let mut bytes_written = 0;

                bytes_written += node.bounds.serialize(&mut sink)?;
                bytes_written += vec![0_usize; node.children.len()].serialize(&mut sink)?;
                bytes_written += node.aggregate.as_ref().as_ref().serialize(&mut sink)?;
                bytes_written += node.data.as_ref().serialize(&mut sink)?;

                bytes_written
            };

            written.insert(node as *const _ as usize, wrote);

            for child in node.children.iter().flatten() {
                queue.push_back(child);
            }
        }

        let mut queue = VecDeque::new();
        queue.push_back(self);

        let mut bytes_written = 0;

        while let Some(node) = queue.pop_front() {
            bytes_written += node.bounds.serialize(writer)?;

            let children = node
                .children
                .iter()
                .map(|i| {
                    i.iter()
                        .map(|j| written[&(j as *const _ as usize)])
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();

            bytes_written += (&children).serialize(writer)?;

            bytes_written += node.aggregate.as_ref().as_ref().serialize(writer)?;
            bytes_written += dbg!(node.data.as_ref().serialize(writer)?);
        }

        Ok(bytes_written)
    }
}

impl<T: Pod> Serialize for &&T {
    fn serialize<W>(&self, writer: &mut W) -> Result<usize>
    where
        W: Write,
    {
        let bytes = bytemuck::bytes_of(**self);
        writer.write_all(bytes)?;

        Ok(bytes.len())
    }
}

impl Serialize for Bounds {
    fn serialize<W>(&self, writer: &mut W) -> Result<usize>
    where
        W: Write,
    {
        let mut bytes_written = 0;

        bytes_written += Serialize::serialize(&self.min(), writer)?;
        bytes_written += Serialize::serialize(&self.max(), writer)?;

        Ok(bytes_written)
    }
}

impl<T> Serialize for Vec<T>
where
    T: Pod,
{
    fn serialize<W>(&self, writer: &mut W) -> Result<usize>
    where
        W: Write,
    {
        <&[T] as Serialize>::serialize(&&self[..], writer)
    }
}

impl<T> Serialize for Option<T>
where
    T: Serialize,
{
    fn serialize<W>(&self, writer: &mut W) -> Result<usize>
    where
        W: Write,
    {
        let mut bytes_written = 0;

        match self {
            Some(inner) => {
                let bytes = bytemuck::bytes_of(&1_usize);
                writer.write_all(bytes)?;
                bytes_written += bytes.len();

                bytes_written += inner.serialize(writer)?;
            }
            _ => {
                let bytes = bytemuck::bytes_of(&0_usize);
                writer.write_all(bytes)?;
                bytes_written += bytes.len();
            }
        }

        Ok(bytes_written)
    }
}

impl<T> Serialize for &Vec<Vec<T>>
where
    T: Pod,
{
    fn serialize<W>(&self, writer: &mut W) -> Result<usize>
    where
        W: Write,
    {
        let mut bytes_written = 0;

        let height = self.len();
        let bytes = bytemuck::bytes_of(&height);
        writer.write_all(&bytes)?;

        bytes_written += bytes.len();

        for v in *self {
            bytes_written += Serialize::serialize(v, writer)?;
        }

        Ok(bytes_written)
    }
}

impl<T> Serialize for &[T]
where
    T: Pod,
{
    fn serialize<W>(&self, writer: &mut W) -> Result<usize>
    where
        W: Write,
    {
        let mut bytes_written = 0;

        {
            let len = self.len();
            let bytes = bytemuck::bytes_of(&len);
            writer.write_all(bytes)?;

            bytes_written += bytes.len();
        }

        {
            let bytes = bytemuck::cast_slice(self);
            writer.write_all(bytes)?;

            bytes_written += bytes.len();
        }

        Ok(bytes_written)
    }
}

impl<T> Serialize for Coord<T>
where
    T: CoordNum + bytemuck::Pod,
{
    fn serialize<W>(&self, writer: &mut W) -> Result<usize>
    where
        W: Write,
    {
        let mut bytes_written = 0;

        {
            let bytes = bytemuck::bytes_of(&self.x);
            writer.write_all(bytes)?;
            bytes_written += bytes.len();
        }
        {
            let bytes = bytemuck::bytes_of(&self.y);
            writer.write_all(bytes)?;
            bytes_written += bytes.len();
        }

        Ok(bytes_written)
    }
}

#[cfg(test)]
mod tests {
    use crate::deserialize::{Deserialize, Pointer};

    use super::*;
    use bytemuck::{Pod, Zeroable};

    #[repr(C)]
    #[derive(Clone, Copy, Default, Pod, Zeroable)]
    struct Test {
        a: f32,
        b: u32,
    }

    impl Test {
        fn new(i: usize) -> Self {
            Self {
                a: i as f32,
                b: i as u32,
            }
        }
    }

    #[test]
    fn write_bytes() {
        let mut buf = Vec::new();

        let t = (0..100).map(Test::new).collect::<Vec<_>>();

        // println!("Initial, len: {}", t.len());

        let bytes = Serialize::serialize(&t.as_slice(), &mut buf).unwrap();
        // println!("Bytes: {}", bytes);

        let (_, rt) = <&[Test] as Deserialize>::deserialize(&buf);
        // println!("Round trip, len: {}", rt.len());

        assert_eq!(rt[50].b, 50);
    }

    #[test]
    fn write_bytes_nested() {
        let mut buf = Vec::new();

        let t = (0..100).map(Test::new).collect::<Vec<_>>();
        let t = vec![t; 20];

        println!("Initial, len: {}", t.len());

        let bytes = <&Vec<Vec<Test>> as Serialize>::serialize(&&t, &mut buf).unwrap();
        println!("Bytes: {}", bytes);

        let (_, rt) = <Vec<&[Test]> as Deserialize>::deserialize(&buf);
        println!("Round trip, len: {}", rt.len());

        assert_eq!(rt[19][50].b, 50);
    }

    #[test]
    fn transmute_pointer() {
        let p = (0..100).collect::<Vec<usize>>();

        let p: &[Pointer<()>] = bytemuck::cast_slice(&p);

        assert_eq!(p[90].position, 90);
    }

    #[test]
    fn read_usize() {
        let bytes = [0, 1, 0, 0, 0, 0, 0, 0];

        let p = *bytemuck::from_bytes::<usize>(&bytes);

        assert_eq!(p, 256);
    }
}
