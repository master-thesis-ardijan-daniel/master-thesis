use std::{
    collections::{HashMap, VecDeque},
    io::{Result, Write},
};

use bytemuck::Pod;
use common::Bounds;
use geo::{Coord, CoordNum};

use crate::TileNode;

pub trait Serialize {
    fn serialize<W>(&self, writer: &mut AlignedWriter<W>) -> Result<()>
    where
        W: Write;
}

#[derive(Clone, Copy)]
pub struct AlignedWriter<W> {
    inner: W,
    position: usize,
}

impl<W> AlignedWriter<W>
where
    W: Write,
{
    pub fn new(inner: W) -> Self {
        Self { inner, position: 0 }
    }

    fn with<W2>(&self, inner: W2) -> AlignedWriter<W2> {
        AlignedWriter {
            inner,
            position: self.position,
        }
    }

    fn write<T>(&mut self, value: &T) -> Result<()>
    where
        T: Pod,
    {
        let padding = self.padding::<T>();
        self.inner.write_all(&vec![0; padding])?;
        self.position += padding;

        let bytes = bytemuck::bytes_of(value);
        self.inner.write_all(bytes)?;
        self.position += bytes.len();

        Ok(())
    }

    fn write_slice<T>(&mut self, slice: &[T]) -> Result<()>
    where
        T: Pod,
    {
        let padding = self.padding::<T>();
        self.inner.write_all(&vec![0; padding])?;
        self.position += padding;

        let bytes = bytemuck::cast_slice(slice);
        self.inner.write_all(bytes)?;
        self.position += bytes.len();

        Ok(())
    }

    pub fn padding<T>(&self) -> usize {
        let alignment = std::mem::align_of::<T>();
        let remainder = self.position % alignment;

        if remainder == 0 {
            0
        } else {
            alignment - remainder
        }
    }
}

impl<T> Serialize for TileNode<T>
where
    T: Pod,
{
    fn serialize<W>(&self, writer: &mut AlignedWriter<W>) -> Result<()>
    where
        W: Write,
    {
        let mut pointers = HashMap::<usize, usize>::new();

        // First pass, calculate pointers
        {
            let mut queue = VecDeque::new();
            queue.push_back(self);

            let mut sink = writer.with(std::io::sink());

            while let Some(node) = queue.pop_front() {
                pointers.insert(node as *const _ as usize, sink.position);

                node.bounds.serialize(&mut sink)?;
                (&node
                    .children
                    .iter()
                    .map(|row| row.iter().map(|_| 0_usize).collect::<Vec<_>>())
                    .collect::<Vec<_>>())
                    .serialize(&mut sink)?;
                node.aggregate.as_ref().as_ref().serialize(&mut sink)?;
                node.data.as_ref().serialize(&mut sink)?;

                for child in node.children.iter().flatten() {
                    queue.push_back(child);
                }
            }
        }

        // Second pass, use pointers from first pass

        let mut queue = VecDeque::new();
        queue.push_back(self);

        while let Some(node) = queue.pop_front() {
            node.bounds.serialize(writer)?;

            let children = node
                .children
                .iter()
                .map(|row| {
                    row.iter()
                        .map(|node| pointers[&(node as *const _ as usize)])
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();

            (&children).serialize(writer)?;

            node.aggregate.as_ref().as_ref().serialize(writer)?;
            node.data.as_ref().serialize(writer)?;

            for child in node.children.iter().flatten() {
                queue.push_back(child);
            }
        }

        Ok(())
    }
}

impl<T: Pod> Serialize for &&T {
    fn serialize<W>(&self, writer: &mut AlignedWriter<W>) -> Result<()>
    where
        W: Write,
    {
        writer.write(**self)
    }
}

impl Serialize for Bounds {
    fn serialize<W>(&self, writer: &mut AlignedWriter<W>) -> Result<()>
    where
        W: Write,
    {
        Serialize::serialize(&self.min(), writer)?;
        Serialize::serialize(&self.max(), writer)?;

        Ok(())
    }
}

impl<T> Serialize for Vec<T>
where
    T: Pod,
{
    fn serialize<W>(&self, writer: &mut AlignedWriter<W>) -> Result<()>
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
    fn serialize<W>(&self, writer: &mut AlignedWriter<W>) -> Result<()>
    where
        W: Write,
    {
        match self {
            Some(inner) => {
                writer.write(&1_u8)?;
                inner.serialize(writer)?;
            }
            _ => {
                writer.write(&0_u8)?;
            }
        }

        Ok(())
    }
}

impl<T> Serialize for &Vec<Vec<T>>
where
    T: Pod,
{
    fn serialize<W>(&self, writer: &mut AlignedWriter<W>) -> Result<()>
    where
        W: Write,
    {
        writer.write(&self.len())?;

        for v in *self {
            Serialize::serialize(v, writer)?;
        }

        Ok(())
    }
}

impl<T> Serialize for &[T]
where
    T: Pod,
{
    fn serialize<W>(&self, writer: &mut AlignedWriter<W>) -> Result<()>
    where
        W: Write,
    {
        writer.write(&self.len())?;
        writer.write_slice(self)?;

        Ok(())
    }
}

impl<T> Serialize for Coord<T>
where
    T: CoordNum + bytemuck::Pod,
{
    fn serialize<W>(&self, writer: &mut AlignedWriter<W>) -> Result<()>
    where
        W: Write,
    {
        writer.write(&self.x)?;
        writer.write(&self.y)?;

        Ok(())
    }
}
