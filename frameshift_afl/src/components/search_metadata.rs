use libafl_bolts::impl_serdeany;
use serde::{Deserialize, Serialize};



#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SearchMetadata {
    pub num_searched: usize,
    pub num_found: usize,

    pub search_tests: usize,
    pub target_time_ms: u64,
    pub total_time_ms: u64,
}

impl SearchMetadata {
    pub fn new() -> Self {
        Self {
            num_searched: 0,
            num_found: 0,
            search_tests: 0,
            target_time_ms: 0,
            total_time_ms: 0,
        }
    }
}

impl_serdeany!(SearchMetadata);
