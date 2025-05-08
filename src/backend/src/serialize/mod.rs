use std::{
    collections::{HashMap, VecDeque},
    io::{Result, Write},
};

use bytemuck::Pod;
use common::Bounds;
use geo::{Coord, CoordNum};

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
        let mut pointers = HashMap::<usize, usize>::new();

        // First pass, calculate pointers
        {
            let mut queue = VecDeque::new();
            queue.push_back(self);

            let mut sink = std::io::sink();

            let mut bytes_written = 0;

            while let Some(node) = queue.pop_front() {
                pointers.insert(node as *const _ as usize, bytes_written);

                bytes_written += node.bounds.serialize(&mut sink)?;
                bytes_written += (&node
                    .children
                    .iter()
                    .map(|row| row.iter().map(|_| 0_usize).collect::<Vec<_>>())
                    .collect::<Vec<_>>())
                    .serialize(&mut sink)?;
                bytes_written += node.aggregate.as_ref().as_ref().serialize(&mut sink)?;
                bytes_written += node.data.as_ref().serialize(&mut sink)?;

                for child in node.children.iter().flatten() {
                    queue.push_back(child);
                }
            }
        }

        // Second pass, use pointers from first pass

        let mut queue = VecDeque::new();
        queue.push_back(self);

        let mut bytes_written = 0;

        while let Some(node) = queue.pop_front() {
            bytes_written += node.bounds.serialize(writer)?;

            let children = node
                .children
                .iter()
                .map(|row| {
                    row.iter()
                        .map(|node| pointers[&(node as *const _ as usize)])
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();

            bytes_written += (&children).serialize(writer)?;

            bytes_written += node.aggregate.as_ref().as_ref().serialize(writer)?;
            bytes_written += node.data.as_ref().serialize(writer)?;

            for child in node.children.iter().flatten() {
                queue.push_back(child);
            }
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
        writer.write_all(bytes)?;

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
