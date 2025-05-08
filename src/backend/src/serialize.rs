use std::{
    collections::VecDeque,
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
        // let mut bytes_written = 0;

        // let mut pointers = Vec::with_capacity(self.children.len());

        // for child in self.children.iter().flatten() {
        //     pointers.push(bytes_written);
        //     bytes_written += Serialize::serialize(child, writer)?;
        // }

        // bytes_written += Serialize::serialize(&self.bounds, writer)?;
        // bytes_written += Serialize::serialize(&pointers, writer)?;

        // bytes_written +=
        //     <Option<&&T> as Serialize>::serialize(&self.aggregate.as_ref().as_ref(), writer)?;
        // bytes_written +=
        //     <Option<&Vec<Vec<T>>> as Serialize>::serialize(&self.data.as_ref(), writer)?;

        // Ok(bytes_written)
        //

        struct NodeInfo<'a, T> {
            node: &'a TileNode<T>,
            offset: usize,
        }

        let mut sink = std::io::sink();

        let mut queue = VecDeque::new();
        let mut offset_info = Vec::new();

        queue.push_back(self);

        let mut pointer = 0;
        while let Some(node) = queue.pop_front() {
            let info = NodeInfo {
                node,
                offset: pointer,
            };

            pointer += node.serialize(&mut sink)?;

            for child in node.children.iter().flatten() {
                queue.push_back(child);
            }

            offset_info.push(info);
        }

        let mut i = 0;
        while i < offset_info.len() {
            let node = offset_info[i].node;

            for child in node.children.iter().flatten() {
                let info = offset_info
                    .iter()
                    .find(|node| std::ptr::eq(node.node as *const _, child as *const _));
            }
        }

        todo!()
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
                writer.write_all(&[0])?;
                bytes_written += 1;

                bytes_written += inner.serialize(writer)?;
            }
            _ => {
                writer.write_all(&[1])?;
                bytes_written += 1;
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
    use crate::deserialize::Deserialize;

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
}
