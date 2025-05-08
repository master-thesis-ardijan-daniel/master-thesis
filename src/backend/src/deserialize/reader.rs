use std::sync::atomic::{AtomicUsize, Ordering};

use bytemuck::Pod;

use super::{
    tree::{Pointer, TileNode},
    Deserialize,
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

        let (read, out) = T::deserialize(&self.data[position..]);

        self.position.fetch_add(read, Ordering::Relaxed);

        out
    }

    pub fn load<T>(&self, pointer: &Pointer<T>) -> TileNode<'a, T>
    where
        T: Pod,
    {
        let (read, out) = Deserialize::deserialize(&self.data[pointer.position..]);

        self.position
            .store(pointer.position + read, Ordering::Relaxed);

        out
    }
}
