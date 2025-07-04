use std::collections::HashSet;

use serde::{Deserialize, Serialize};


#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Structured {
    pub raw: Vec<u8>,
    pub relations: Vec<Relation>,
}

impl Structured {
    pub fn raw(raw: Vec<u8>) -> Self {
        Self {
            raw,
            relations: Vec::new()
        }
    }

    pub fn add_relation(&mut self, rel: Relation) {
        self.relations.push(rel);
    }

    pub fn get_raw_mut(&mut self) -> &mut [u8] {
        &mut self.raw
    }

    pub fn get_raw(&self) -> &[u8] {
        &self.raw
    }

    pub fn write(&mut self, idx: usize, data: &[u8]) {
        self.raw.splice(idx..idx+data.len(), data.iter().cloned());
        self.sanitize();
    }

    pub fn insert(&mut self, idx: usize, data: &[u8]) -> Result<(),()> {
        for rel in self.relations.iter_mut() {
            if !rel.enabled {
                continue;
            }

            if rel.on_insert(idx, data.len()).is_err() {
                return Err(());
            }
        }

        self.raw.splice(idx..idx, data.iter().cloned());

        self.sanitize();

        Ok(())
    }

    // Track an insert without modifying a buffer.
    pub fn on_insert(&mut self, idx: usize, size: usize) -> Result<(),()> {
        for rel in self.relations.iter_mut() {
            if !rel.enabled {
                continue;
            }

            if rel.on_insert(idx, size).is_err() {
                return Err(());
            }
        }

        Ok(())
    }

    pub fn insert_ignore_invalid(&mut self, idx: usize, data: &[u8]) {
        for rel in self.relations.iter_mut() {
            if !rel.enabled {
                continue;
            }

            if rel.on_insert(idx, data.len()).is_err() {
                // Ignore
            }
        }

        self.raw.splice(idx..idx, data.iter().cloned());

        self.sanitize();
    }

    pub fn remove(&mut self, idx: usize, size: usize) -> Result<(),()> {
        for rel in self.relations.iter_mut() {
            if !rel.enabled {
                continue;
            }

            if rel.on_remove(idx, size).is_err() {
                return Err(());
            }
        }

        self.raw.drain(idx..idx + size);

        self.sanitize();

        Ok(())
    }

    pub fn insert_disabling(&mut self, idx: usize, data: &[u8]) {
        let mut disabled = vec![];
        for (i, rel) in self.relations.iter_mut().enumerate() {
            if !rel.enabled {
                continue;
            }

            if rel.on_insert(idx, data.len()).is_err() {
                disabled.push(i);
            }
        }

        self.raw.splice(idx..idx, data.iter().cloned());

        for i in disabled.iter().rev() {
            self.relations.swap_remove(*i);
        }

        self.sanitize();
    }

    pub fn remove_disabling(&mut self, idx: usize, size: usize) {
        let mut disabled = vec![];
        for (i, rel) in self.relations.iter_mut().enumerate() {
            if !rel.enabled {
                continue;
            }

            if rel.on_remove(idx, size).is_err() {
                disabled.push(i);
            }
        }

        self.raw.drain(idx..idx + size);

        for i in disabled.iter().rev() {
            self.relations.swap_remove(*i);
        }

        self.sanitize();
    }

    pub fn sanitize(&mut self) {
        for rel in self.relations.iter() {
            if !rel.enabled {
                continue;
            }

            rel.apply(self.raw.as_mut());
        }
    }

    pub fn sanitize_buffer(&self, buf: &mut [u8]) {
        for rel in self.relations.iter() {
            if !rel.enabled {
                continue;
            }

            rel.apply(buf);
        }
    }

    pub fn inflection_points(&self) -> HashSet<usize> {
        let mut points = HashSet::new();
        for rel in self.relations.iter() {
            // Only use 4 and 8 byte fields as indirect pointers.
            if rel.size == 4 || rel.size == 8 {
                points.insert(rel.pos);
                points.insert(rel.anchor);
                points.insert(rel.insert);
            }
        }
        points
    }

    pub fn insertion_points(&self) -> Vec<usize> {
        let mut points = HashSet::new();
        points.insert(self.raw.len());
        for rel in self.relations.iter() {
            points.insert(rel.insert);
        }
        points.into_iter().collect()
    }

    pub fn set_relation_enabled(&mut self, idx: usize, enabled: bool) {
        self.relations[idx].enabled = enabled;
    }

    pub fn save_relations(&mut self) {
        for rel in self.relations.iter_mut() {
            rel.save();
        }
    }

    pub fn restore_relations(&mut self) {
        for rel in self.relations.iter_mut() {
            rel.restore();
        }
    }
}


#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Relation {
    pub pos: usize,
    pub value: u64,
    pub size: usize,
    pub le: bool,
    pub anchor: usize,
    pub insert: usize,

    /// Used during validation to efficiently turn off relations that are invalid.
    pub enabled: bool,

    /// Used to restore the relation to its previous state.
    pub old_pos: usize,
    pub old_anchor: usize,
    pub old_insert: usize,
    pub old_value: u64,
}


impl Relation {
    pub fn new(pos: usize, value: u64, size: usize, le: bool, anchor: usize, insert: usize) -> Self {
        Self {
            pos,
            value,
            size,
            le,
            anchor,
            insert,
            enabled: true,
            old_pos: pos,
            old_anchor: anchor,
            old_insert: insert,
            old_value: value,
        }
    }

    pub fn on_insert(&mut self, idx: usize, size: usize) -> Result<(),()> {
        // Error if insert is inside the field.
        if idx > self.pos && idx < self.pos + self.size {
            return Err(());
        }

        // Check if we should update the value of the field.
        if idx >= self.anchor && idx <= self.insert {
            self.value += size as u64;

            // Check if we've overflowed the field.
            let max_val = match &self.size {
                1 => 0xff,
                2 => 0xffff,
                3 => 0xffffff,
                4 => 0xffffffff,
                8 => 0xffffffffffffffff,
                _ => panic!("Unsupported size")
            };

            if self.value > max_val {
                return Err(());
            }
        }

        // Move the field.
        if idx <= self.pos {
            self.pos += size;
        }

        // Move the anchor point.
        // Anchor point of 0 is locked.
        if idx < self.anchor {
            self.anchor += size;
        }

        // Move the insert point.
        if idx <= self.insert {
            self.insert += size;
        }

        Ok(())
    }

    pub fn on_remove(&mut self, idx: usize, size: usize) -> Result<(),()> {
        // Error if remove overlaps the field.
        if idx < self.pos + self.size && idx + size > self.pos {
            return Err(());
        }

        let pre_pos = if idx < self.pos {
            (self.pos - idx).min(size)
        } else {
            0
        };

        let pre_anchor = if idx < self.anchor {
            (self.anchor - idx).min(size)
        } else {
            0
        };

        let pre_insert = if idx < self.insert {
            (self.insert - idx).min(size)
        } else {
            0
        };

        let overlap_min = idx.clamp(self.anchor, self.insert);
        let overlap_max = (idx + size).clamp(self.anchor, self.insert);

        let insert_overlap = overlap_max - overlap_min;

        // Adjust the field value.
        if (insert_overlap as u64) > self.value {
            return Err(());
        } else {
            self.value -= insert_overlap as u64;
        }

        // Adjust positions.
        self.pos -= pre_pos;
        self.anchor -= pre_anchor;
        self.insert -= pre_insert;

        Ok(())

    }

    pub fn apply(&self, input: &mut [u8]) {
        // Write the value of the field to the input
        let byt = match (&self.size, &self.le) {
            (1, _) => (self.value as u8).to_le_bytes().to_vec(),
            (2, true) => (self.value as u16).to_le_bytes().to_vec(),
            (2, false) => (self.value as u16).to_be_bytes().to_vec(),
            (3, true) => (self.value as u32).to_le_bytes()[0..3].to_vec(),
            (3, false) => (self.value as u32).to_be_bytes()[1..4].to_vec(),
            (4, true) => (self.value as u32).to_le_bytes().to_vec(),
            (4, false) => (self.value as u32).to_be_bytes().to_vec(),
            (8, true) => (self.value as u64).to_le_bytes().to_vec(),
            (8, false) => (self.value as u64).to_be_bytes().to_vec(),
            _ => panic!("Unsupported size")
        };

        for i in 0..self.size {
            input[self.pos + i] = byt[i];
        }
    }

    pub fn save(&mut self) {
        self.old_pos = self.pos;
        self.old_anchor = self.anchor;
        self.old_insert = self.insert;
        self.old_value = self.value;
    }

    pub fn restore(&mut self) {
        self.pos = self.old_pos;
        self.anchor = self.old_anchor;
        self.insert = self.old_insert;
        self.value = self.old_value;
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_size1() {
        // ....FFFF|........|....
        let base = Relation::new(4, 8, 4, true, 8, 16);

        let mut rel = base.clone();
        assert!(rel.on_insert(0, 1).is_ok());
        assert_eq!(rel.pos, 5);
        assert_eq!(rel.anchor, 9);
        assert_eq!(rel.insert, 17);
        assert_eq!(rel.value, 8);

        let mut rel = base.clone();
        assert!(rel.on_insert(4, 1).is_ok());
        assert_eq!(rel.pos, 5);
        assert_eq!(rel.anchor, 9);
        assert_eq!(rel.insert, 17);
        assert_eq!(rel.value, 8);

        let mut rel = base.clone();
        assert!(rel.on_insert(5, 1).is_err());

        let mut rel = base.clone();
        assert!(rel.on_insert(8, 1).is_ok());
        assert_eq!(rel.pos, 4);
        assert_eq!(rel.anchor, 8);
        assert_eq!(rel.insert, 17);
        assert_eq!(rel.value, 9);

        let mut rel = base.clone();
        assert!(rel.on_insert(12, 1).is_ok());
        assert_eq!(rel.pos, 4);
        assert_eq!(rel.anchor, 8);
        assert_eq!(rel.insert, 17);
        assert_eq!(rel.value, 9);
    }

    #[test]
    fn test_insert_size2() {
        // ....FFFF....|........|....
        let base = Relation::new(4, 8, 4, true, 12, 20);

        let mut rel = base.clone();
        assert!(rel.on_insert(0, 1).is_ok());
        assert_eq!(rel.pos, 5);
        assert_eq!(rel.anchor, 13);
        assert_eq!(rel.insert, 21);
        assert_eq!(rel.value, 8);

        let mut rel = base.clone();
        assert!(rel.on_insert(4, 1).is_ok());
        assert_eq!(rel.pos, 5);
        assert_eq!(rel.anchor, 13);
        assert_eq!(rel.insert, 21);
        assert_eq!(rel.value, 8);

        let mut rel = base.clone();
        assert!(rel.on_insert(5, 1).is_err());

        let mut rel = base.clone();
        assert!(rel.on_insert(8, 1).is_ok());
        assert_eq!(rel.pos, 4);
        assert_eq!(rel.anchor, 13);
        assert_eq!(rel.insert, 21);
        assert_eq!(rel.value, 8);

        let mut rel = base.clone();
        assert!(rel.on_insert(12, 1).is_ok());
        assert_eq!(rel.pos, 4);
        assert_eq!(rel.anchor, 12);
        assert_eq!(rel.insert, 21);
        assert_eq!(rel.value, 9);
    }

    #[test]
    fn test_insert_offset() {
        // |....FFFF....|....
        let base = Relation::new(4, 12, 4, true, 0, 12);

        let mut rel = base.clone();
        assert!(rel.on_insert(0, 1).is_ok());
        assert_eq!(rel.pos, 5);
        assert_eq!(rel.anchor, 0);
        assert_eq!(rel.insert, 13);
        assert_eq!(rel.value, 13);

        let mut rel = base.clone();
        assert!(rel.on_insert(4, 1).is_ok());
        assert_eq!(rel.pos, 5);
        assert_eq!(rel.anchor, 0);
        assert_eq!(rel.insert, 13);
        assert_eq!(rel.value, 13);

        let mut rel = base.clone();
        assert!(rel.on_insert(5, 1).is_err());

        let mut rel = base.clone();
        assert!(rel.on_insert(8, 1).is_ok());
        assert_eq!(rel.pos, 4);
        assert_eq!(rel.anchor, 0);
        assert_eq!(rel.insert, 13);
        assert_eq!(rel.value, 13);

        let mut rel = base.clone();
        assert!(rel.on_insert(12, 1).is_ok());
        assert_eq!(rel.pos, 4);
        assert_eq!(rel.anchor, 0);
        assert_eq!(rel.insert, 13);
        assert_eq!(rel.value, 13);
    }

    #[test]
    fn test_remove_size1() {
        // ....FFFF|........|....
        let base = Relation::new(4, 8, 4, true, 8, 16);

        let mut rel = base.clone();
        assert!(rel.on_remove(0, 1).is_ok());
        assert_eq!(rel.pos, 3);
        assert_eq!(rel.anchor, 7);
        assert_eq!(rel.insert, 15);
        assert_eq!(rel.value, 8);

        let mut rel = base.clone();
        assert!(rel.on_remove(4, 1).is_err());

        let mut rel = base.clone();
        assert!(rel.on_remove(7, 1).is_err());

        let mut rel = base.clone();
        assert!(rel.on_remove(8, 1).is_ok());
        assert_eq!(rel.pos, 4);
        assert_eq!(rel.anchor, 8);
        assert_eq!(rel.insert, 15);
        assert_eq!(rel.value, 7);

        let mut rel = base.clone();
        assert!(rel.on_remove(12, 1).is_ok());
        assert_eq!(rel.pos, 4);
        assert_eq!(rel.anchor, 8);
        assert_eq!(rel.insert, 15);
        assert_eq!(rel.value, 7);

        let mut rel = base.clone();
        assert!(rel.on_remove(16, 1).is_ok());
        assert_eq!(rel.pos, 4);
        assert_eq!(rel.anchor, 8);
        assert_eq!(rel.insert, 16);
        assert_eq!(rel.value, 8);
    }

    #[test]
    fn test_remove_size2() {
        // ....FFFF....|........|....
        let base = Relation::new(4, 8, 4, true, 12, 20);

        let mut rel = base.clone();
        assert!(rel.on_remove(0, 1).is_ok());
        assert_eq!(rel.pos, 3);
        assert_eq!(rel.anchor, 11);
        assert_eq!(rel.insert, 19);
        assert_eq!(rel.value, 8);

        let mut rel = base.clone();
        assert!(rel.on_remove(4, 1).is_err());

        let mut rel = base.clone();
        assert!(rel.on_remove(7, 1).is_err());

        let mut rel = base.clone();
        assert!(rel.on_remove(8, 1).is_ok());
        assert_eq!(rel.pos, 4);
        assert_eq!(rel.anchor, 11);
        assert_eq!(rel.insert, 19);
        assert_eq!(rel.value, 8);

        let mut rel = base.clone();
        assert!(rel.on_remove(12, 1).is_ok());
        assert_eq!(rel.pos, 4);
        assert_eq!(rel.anchor, 12);
        assert_eq!(rel.insert, 19);
        assert_eq!(rel.value, 7);
    }

    #[test]
    fn test_remove_offset() {
        // |....FFFF....|....
        let base = Relation::new(4, 12, 4, true, 0, 12);

        let mut rel = base.clone();
        assert!(rel.on_remove(0, 1).is_ok());
        assert_eq!(rel.pos, 3);
        assert_eq!(rel.anchor, 0);
        assert_eq!(rel.insert, 11);
        assert_eq!(rel.value, 11);

        let mut rel = base.clone();
        assert!(rel.on_remove(4, 1).is_err());

        let mut rel = base.clone();
        assert!(rel.on_remove(7, 1).is_err());

        let mut rel = base.clone();
        assert!(rel.on_remove(8, 1).is_ok());
        assert_eq!(rel.pos, 4);
        assert_eq!(rel.anchor, 0);
        assert_eq!(rel.insert, 11);
        assert_eq!(rel.value, 11);

        let mut rel = base.clone();
        assert!(rel.on_remove(12, 1).is_ok());
        assert_eq!(rel.pos, 4);
        assert_eq!(rel.anchor, 0);
        assert_eq!(rel.insert, 12);
        assert_eq!(rel.value, 12);
    }

    #[test]
    fn roundtrip() {
        let rels = vec![
            Relation::new(4, 8, 4, true, 8, 16),
            Relation::new(4, 8, 4, true, 12, 20),
            Relation::new(4, 12, 4, true, 0, 12)
        ];

        for base in rels {
            for i in 0..20 {
                for size in 1..5 {
                    let mut rel = base.clone();
                    if rel.on_insert(i, size).is_ok() {
                        let mut rel2 = rel.clone();
                        assert!(rel2.on_remove(i, size).is_ok());
                        assert_eq!(rel2, base);
                    }
                }
            }
        }
    }

    #[test]
    fn test_oob_relation() {
        let mut rel = Relation::new(0, 0x30, 1, true, 0, 1);
        assert!(rel.on_insert(0, 0x40).is_ok());
        assert!(rel.on_insert(1, 0xf0).is_err());
    }
}
