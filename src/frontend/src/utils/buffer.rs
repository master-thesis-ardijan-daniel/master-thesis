use std::{
    collections::{HashMap, HashSet, VecDeque},
    ops::Deref,
};

use common::Bounds;
use geo::Coord;

#[derive(Debug, Copy, Clone)]
pub struct BufferSlot(pub usize);

// impl AsRef<u32> for BufferSlot {
//     fn as_ref(&self) -> &u32 {
//         &self.0
//     }
// }

impl Deref for BufferSlot {
    type Target = usize;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug)]
pub struct Level {
    step_x: f32,
    step_y: f32,
}

impl Level {
    pub fn new(bounds: Bounds, width: usize, height: usize) -> Self {
        let step_x = bounds.width() / width as f32;
        let step_y = bounds.height() / height as f32;

        Self { step_x, step_y }
    }
}

#[derive(Debug)]
pub struct BufferAllocator {
    levels: Vec<Level>,
    pub current_level: usize,

    visible: HashSet<(u32, u32, u32)>,
    allocated: HashMap<(u32, u32, u32), BufferSlot>,

    marked: VecDeque<(u32, u32, u32)>,
    free: VecDeque<BufferSlot>,
}

impl BufferAllocator {
    pub fn new(levels: Vec<Level>, slots: usize) -> Self {
        let free = (0..slots).map(BufferSlot).collect();

        Self {
            levels,
            current_level: 0,

            visible: HashSet::new(),
            free,
            marked: VecDeque::new(),
            allocated: HashMap::new(),
        }
    }

    pub fn allocate(&mut self, zoom: u32, points: &[Coord<f32>]) -> Vec<(u32, u32, u32)> {
        let level = &self.levels[self.current_level];

        let mut visible = HashSet::new();

        for point in points {
            visible.insert((
                zoom,
                ((90. - point.y) / level.step_y).floor() as u32,
                ((180. + point.x) / level.step_x).floor() as u32,
            ));
        }

        // Remove tiles from deallocation queue if they are
        // now visible again.
        self.marked.retain(|tile| !visible.contains(tile));

        // Visible in current frame, but not for the current
        // zoom level. We want to queue these tiles available
        // for slot stealing.
        let visible_but_different_zoom_level = self
            .visible
            .iter()
            .filter(|(z, _, _)| (*z as usize) < self.current_level)
            .collect::<HashSet<_>>();

        if visible.len() < 5 {
            self.current_level = (self.current_level + 1).min(self.levels.len().saturating_sub(1));
        }

        if visible.len() > 10 {
            self.current_level = self.current_level.saturating_sub(1);
        }

        // Tiles visible in previous frame, but not visible in current frame
        let not_visible_anymore = self.visible.difference(&visible).collect::<HashSet<_>>();

        // Mark tiles available for slot stealing
        let to_be_marked = not_visible_anymore.union(&visible_but_different_zoom_level);

        for &&tile in to_be_marked {
            self.marked.push_back(tile);
        }

        let mut to_be_fetched = Vec::new();

        for &tile in &visible {
            if self.allocated.contains_key(&tile) {
                // Tile is already allocated
                continue;
            }

            let slot = self.free.pop_front().or_else(|| {
                // No free slot found, steal a slot

                self.marked
                    .pop_front()
                    .and_then(|tile| self.allocated.remove(&tile))
            });

            if let Some(slot) = slot {
                self.allocated.insert(tile, slot);
                to_be_fetched.push(tile);
            }
        }

        self.visible = visible;

        to_be_fetched
    }

    pub fn slot(&self, tile: &(u32, u32, u32)) -> Option<&BufferSlot> {
        self.allocated.get(tile)
    }
}
