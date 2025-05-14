use std::sync::atomic::{AtomicUsize, Ordering};

use bytemuck::Pod;

use super::{
    tree::{Pointer, TileNode},
    AlignedReader, Deserialize,
};

pub struct Reader<'a> {
    pub data: &'a [u8],
    pub position: AtomicUsize,
}

impl<'a> Reader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            position: AtomicUsize::new(0),
        }
    }

    pub fn read<T>(&self) -> T
    where
        T: Deserialize<'a>,
    {
        let position = self.position.load(Ordering::Relaxed);

        let mut reader = AlignedReader::new(&self.data[position..]);
        let out = T::deserialize(&mut reader);
        self.position.fetch_add(reader.position, Ordering::Relaxed);

        out
    }

    pub fn load<T>(&self, pointer: &Pointer<T>) -> TileNode<'a, T>
    where
        T: Pod,
    {
        let mut reader = AlignedReader::new(&self.data[pointer.position..]);
        let out = Deserialize::deserialize(&mut reader);

        self.position
            .store(pointer.position + reader.position, Ordering::Relaxed);

        out
    }
}
