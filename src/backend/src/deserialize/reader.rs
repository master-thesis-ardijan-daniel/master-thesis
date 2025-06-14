use bytemuck::Pod;

use super::{
    tree::{Pointer, TileNode},
    AlignedReader, Deserialize,
};

pub struct Reader<'a> {
    pub data: &'a [u8],
    pub position: usize,
}

impl<'a> Reader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, position: 0 }
    }

    pub fn read<T>(&mut self) -> T
    where
        T: Deserialize<'a>,
    {
        let mut reader = AlignedReader::new(&self.data[self.position..]);
        let out = T::deserialize(&mut reader);

        self.position += reader.position;

        out
    }

    pub fn load<T>(&mut self, pointer: &Pointer<T>) -> TileNode<'a, T>
    where
        T: Pod,
    {
        let mut reader = AlignedReader::new(&self.data[pointer.position..]);
        let out = Deserialize::deserialize(&mut reader);

        self.position = pointer.position + reader.position;

        out
    }
}
