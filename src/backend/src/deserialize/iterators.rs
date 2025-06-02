use crate::{deserialize::reader::Reader, Bounds};
use bytemuck::Pod;
use geo::{Contains, Intersects};

use super::tree::{Pointer, TileData};

pub struct ContainsIterator<'a, DataType, AggregateType, Query> {
    query: Query,

    reader: Reader<'a>,
    queue: Vec<Pointer<DataType>>,

    _marker: std::marker::PhantomData<fn() -> AggregateType>,
}

impl<'a, DataType, AggregateType, Query> ContainsIterator<'a, DataType, AggregateType, Query> {
    pub fn new(reader: Reader<'a>, query: Query) -> Self {
        let queue = vec![Pointer::default()];

        Self {
            query,
            reader,
            queue,
            _marker: Default::default(),
        }
    }
}

impl<'a, DataType, AggregateType, Query> Iterator
    for ContainsIterator<'a, DataType, AggregateType, Query>
where
    DataType: Pod,
    AggregateType: Pod,
    Query: Contains<Bounds> + Intersects<Bounds>,
{
    type Item = TileData<'a, DataType, AggregateType>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(current) = self.queue.pop() {
            let node = self.reader.load(&current);

            if self.query.contains(&node.bounds) {
                return Some(self.reader.read());
            }

            if self.query.intersects(&node.bounds) {
                self.queue.extend(node.children.into_iter().flatten());
            }
        }

        None
    }
}
