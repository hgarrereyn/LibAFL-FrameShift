use ahash::RandomState;
use libafl::{corpus::CorpusId, inputs::{HasMutatorBytes, HasTargetBytes, Input}, Error};
use libafl_bolts::{fs::write_file_atomic, prelude::OwnedSlice, HasLen};
use rand::{rngs::StdRng, Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use std::{hash::{BuildHasher, Hasher}, io::Read, path::Path};
use std::fmt::Debug;

use crate::core::structured::Structured;


#[derive(Serialize, Deserialize, Clone)]
pub struct StructuredInput {
    pub input: Structured,
    pub status: InputStatus,
    pub seed: u64,
}

impl Debug for StructuredInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StructuredInput")
            .field("status", &self.status)
            .finish()
    }
}

impl StructuredInput {
    pub fn new_raw(bytes: &[u8]) -> Self {
        Self {
            input: Structured::raw(bytes.to_vec()),
            status: InputStatus::New,
            seed: 0,
        }
    }

    pub fn new_structured(input: Structured) -> Self {
        Self {
            input,
            status: InputStatus::New,
            seed: 0,
        }
    }

    pub fn set_seed(&mut self, seed: u64) {
        self.seed = seed;
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum InputStatus {
    /// New grammar, has not been searched.
    New,

    /// Mutated from a grammar which has been searched.
    Mutated,

    /// Sometimes we will crash during the search, so we mark inputs as in progress to avoid falling into a loop.
    InProgress,

    /// A searched grammar (corpus entry should match the entry here).
    Searched(CorpusId),
}

impl Input for StructuredInput {
    fn generate_name(&self, _idx: usize) -> String {
        let mut hasher = RandomState::with_seeds(0, 0, 0, 0).build_hasher();
        hasher.write(&self.input.get_raw());
        format!("{:016x}", hasher.finish())
    }

    fn to_file<P>(&self, path: P) -> Result<(), Error>
    where
        P: AsRef<Path>,
    {
        // Write raw data to file
        write_file_atomic(&path, &self.input.get_raw())?;

        let parent = path.as_ref().parent().unwrap();
        let file_name = path.as_ref().file_name().unwrap();
        let full_path = parent.join(format!(".{}.annotated", file_name.to_string_lossy()));

        // Write annotated data to file
        let json = serde_json::to_string(&self.input).unwrap();
        write_file_atomic(full_path, json.as_bytes())?;

        Ok(())
    }

    /// Load the content of this input from a file
    fn from_file<P>(path: P) -> Result<Self, Error>
    where
        P: AsRef<Path>,
    {
        let parent = path.as_ref().parent().unwrap();
        let file_name = path.as_ref().file_name().unwrap();
        let full_path = parent.join(format!(".{}.annotated", file_name.to_string_lossy()));

        // Check if annotated file exists
        if full_path.exists() {
            // Load annotated data
            let json = std::fs::read_to_string(full_path)?;
            let structure: Structured = serde_json::from_str(&json)?;

            Ok(StructuredInput::new_structured(structure))
        } else {
            // Load raw data
            let mut file = std::fs::File::open(path)?;
            let mut bytes = vec![];
            file.read_to_end(&mut bytes)?;

            Ok(StructuredInput::new_raw(&bytes))
        }
    }
}

impl HasLen for StructuredInput {
    fn len(&self) -> usize {
        self.input.get_raw().len()
    }
}

impl HasTargetBytes for StructuredInput {
    fn target_bytes(&self) -> OwnedSlice<u8> {
        let x = self.input.get_raw();
        OwnedSlice::from(x)
    }
}

impl HasMutatorBytes for StructuredInput {
    fn bytes(&self) -> &[u8] {
        self.input.get_raw()
    }

    fn bytes_mut(&mut self) -> &mut [u8] {
        self.input.get_raw_mut()
    }

    fn resize(&mut self, new_len: usize, value: u8) {
        let mut rng = StdRng::seed_from_u64(self.seed);

        let prev_len = self.input.get_raw().len();

        if new_len > prev_len {
            let diff = new_len - prev_len;
            let data = vec![value; diff];

            // Find insertion point.
            let insertions = self.input.insertion_points();
            let insert_pos = insertions[rng.gen_range(0..insertions.len())];
            self.input.insert_disabling(insert_pos, &data);
        } else if new_len < prev_len {
            self.input.remove_disabling(new_len, prev_len - new_len);
        }
    }

    fn extend<'a, I: IntoIterator<Item = &'a u8>>(&mut self, iter: I) {
        let data = iter.into_iter().cloned().collect::<Vec<_>>();
        self.input.insert_disabling(self.input.get_raw().len(), &data);
    }

    fn splice<R, I>(&mut self, range: R, replace_with: I) -> Option<std::vec::Splice<'_, I::IntoIter>>
    where
        R: std::ops::RangeBounds<usize>,
        I: IntoIterator<Item = u8> {    
        let start = match range.start_bound() {
            std::ops::Bound::Included(&x) => x,
            std::ops::Bound::Excluded(&x) => x + 1,
            std::ops::Bound::Unbounded => 0,
        };

        let end = match range.end_bound() {
            std::ops::Bound::Included(&x) => x + 1,
            std::ops::Bound::Excluded(&x) => x,
            std::ops::Bound::Unbounded => self.input.get_raw().len(),
        };

        let replace_with = replace_with.into_iter().collect::<Vec<_>>();

        let prev_size = end - start;
        let new_size = replace_with.len();

        if prev_size == new_size {
            self.input.write(start, &replace_with);
        } else if prev_size > new_size {
            self.input.write(start, &replace_with);
            self.input.remove_disabling(start + new_size, prev_size - new_size);
        } else {
            self.input.write(start, &replace_with[..prev_size]);
            self.input.insert_disabling(end, &replace_with[prev_size..]);
        }

        None
    }

    fn drain<R>(&mut self, range: R) -> Option<std::vec::Drain<'_, u8>>
    where
        R: std::ops::RangeBounds<usize> {
        let start = match range.start_bound() {
            std::ops::Bound::Included(&x) => x,
            std::ops::Bound::Excluded(&x) => x + 1,
            std::ops::Bound::Unbounded => 0,
        };

        let end = match range.end_bound() {
            std::ops::Bound::Included(&x) => x + 1,
            std::ops::Bound::Excluded(&x) => x,
            std::ops::Bound::Unbounded => self.input.get_raw().len(),
        };

        self.input.remove_disabling(start, end - start);

        None
    }
}
