// Copyright 2016-2017 Matthew D. Michelotti
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::f64;
use std::collections::hash_map;
use fnv::{FnvHashMap, FnvHashSet};
use std::cmp;
use core::{HitboxId, Hitbox};
use core::dur_hitbox::DurHitbox;
use core::inter::Group;
use util::TightSet;
use index_rect::IndexRect;
use geom::shape::PlacedBounds;

// Grid is a sparse 2D grid implemented as a HashMap. This is used as the
// pruning method to decide which hitboxes to check for collisions.

//TODO add unit tests for Grid

#[derive(PartialEq, Eq, Copy, Clone, Hash)]
struct GridKey {
    coord: (i32, i32),
    group: Group
}

#[derive(Copy, Clone)]
struct GridArea {
    rect: IndexRect,
    group: Group
}

impl GridArea {
    fn contains(&self, key: GridKey) -> bool {
        self.group == key.group && self.rect.contains(key.coord)
    }
}

pub struct Grid {
    map: FnvHashMap<GridKey, TightSet<HitboxId>>,
    cell_width: f64
}

impl Grid {
    pub fn new(cell_width: f64) -> Grid {
        Grid { map : FnvHashMap::default(), cell_width: cell_width }
    }

    pub fn cell_period(&self, hitbox: &Hitbox, has_group: bool) -> f64 {
        if has_group {
            let speed = hitbox.vel.max_edge();
            if speed <= 0.0 {
                f64::INFINITY
            } else {
                self.cell_width / speed
            }
        } else {
            f64::INFINITY
        }
    }

    pub fn update_hitbox(&mut self, hitbox_id: HitboxId, group: Group, old_hitbox: Option<&DurHitbox>,
                         new_hitbox: Option<&DurHitbox>, groups: &[Group]) -> Option<FnvHashSet<HitboxId>>
    {
        assert!(new_hitbox.is_some() || groups.is_empty());
        let old_area = old_hitbox.map(|old_hitbox| self.index_bounds(old_hitbox, group));
        let new_area = new_hitbox.map(|new_hitbox| self.index_bounds(new_hitbox, group));
        self.update_area(hitbox_id, old_area, new_area);
        new_area.map(|new_area| self.overlapping_ids(hitbox_id, new_area.rect, groups))
    }

    fn index_bounds(&self, hitbox: &DurHitbox, group: Group) -> GridArea {
        let bounds = hitbox.bounding_box();
        let start_x = (bounds.left() / self.cell_width).floor() as i32;
        let start_y = (bounds.bottom() / self.cell_width).floor() as i32;
        let end_x = cmp::max((bounds.right() / self.cell_width).ceil() as i32, start_x + 1);
        let end_y = cmp::max((bounds.top() / self.cell_width).ceil() as i32, start_y + 1);
        GridArea { rect : IndexRect::new((start_x, start_y), (end_x, end_y)), group : group }
    }

    fn overlapping_ids(&self, hitbox_id: HitboxId, rect: IndexRect, groups: &[Group]) -> FnvHashSet<HitboxId> {
        let mut result = FnvHashSet::default();
        for &group in groups {
            for coord in rect.iter() {
                let key = GridKey { coord : coord, group : group };
                if let Some(other_ids) = self.map.get(&key) {
                    for &other_id in other_ids.iter() {
                        if other_id != hitbox_id { result.insert(other_id); }
                    }
                }
            }
        }
        result
    }

    fn update_area(&mut self, hitbox_id: HitboxId, old_area: Option<GridArea>, new_area: Option<GridArea>) {
        if let Some(old_area) = old_area {
            for coord in old_area.rect.iter() {
                let key = GridKey { coord : coord, group : old_area.group };
                if new_area.map_or(true, |new_area| !new_area.contains(key)) {
                    if let hash_map::Entry::Occupied(mut entry) = self.map.entry(key) {
                        let success = entry.get_mut().remove(&hitbox_id);
                        assert!(success, "illegal state");
                        if entry.get().is_empty() { entry.remove(); }
                    } else {
                        unreachable!();
                    }
                }
            }
        }
        if let Some(new_area) = new_area {
            for coord in new_area.rect.iter() {
                let key = GridKey { coord : coord, group : new_area.group };
                if old_area.map_or(true, |old_area| !old_area.contains(key)) {
                   let other_ids = self.map.entry(key).or_insert_with(|| TightSet::new());
                   let success = other_ids.insert(hitbox_id);
                   assert!(success, "illegal state");
                }
            }
        }
    }
}
