use std::{cell::RefCell, collections::HashSet};

use colored::Colorize;

use super::structured::{Relation, Structured};


#[derive(Debug, Clone)]
pub struct SearchOptions {
    pub verbose: bool,
    pub extra_verbose: bool,
    pub max_iters: usize,

    // Thresholds.
    pub loss_threshold: f64,
    pub recover_threshold: f64,
}

impl SearchOptions {
    pub fn verbose() -> Self {
        Self {
            verbose: true,
            ..Default::default()
        }
    }
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            verbose: false,
            extra_verbose: false,
            max_iters: 10,
            loss_threshold: 0.05,
            recover_threshold: 0.2,
        }
    }
}

pub struct SearchContext<'o,O> {
    oracle: RefCell<&'o mut O>,
    pub options: SearchOptions,
    pub focus_indices: Vec<usize>,
    pub loss_threshold: usize,
    pub test_count: RefCell<usize>,
    pub target_test_ms: RefCell<u64>
}

pub struct SearchResult {
    pub input: Structured,
    pub test_count: usize,
    pub target_test_ms: u64,
    pub total_test_ms: u64,
    pub found_any: bool
}

impl<'o,O> SearchContext<'o,O>
where
    O: FnMut(&[u8]) -> &'o [u8],
{
    pub fn new(testcase: &Structured, oracle: &'o mut O, options: SearchOptions) -> Self {
        // What coverage does the current test case get?
        let seed_cov = oracle(&testcase.get_raw());

        let mut seed_indices = Vec::with_capacity(seed_cov.len());
        for idx in 0..seed_cov.len() {
            if seed_cov[idx] != 0 {
                seed_indices.push(idx);
            }
        }

        // What coverage does an empty test case get (i.e. max loss)?
        let base_cov = oracle(&[]);

        // Pick out the interesting indices (found by current test case, but not by base case).
        let mut focus_indices = Vec::with_capacity(seed_indices.len());
        for idx in seed_indices.iter() {
            if base_cov[*idx] == 0 {
                focus_indices.push(*idx);
            }
        }

        if options.extra_verbose {
            println!("seed_indices: {:?}", seed_indices);
            println!("focus_indices: {:?}", focus_indices);
        }

        // theta_0 = 5% of the losable coverage (at least 1 feature)
        let loss_threshold = ((options.loss_threshold * focus_indices.len() as f64).ceil() as usize).max(1);

        Self {
            oracle: RefCell::new(oracle),
            options,
            focus_indices,
            loss_threshold,
            test_count: RefCell::new(0),
            target_test_ms: RefCell::new(0)
        }
    }

    pub fn search(testcase: &Structured, oracle: &'o mut O, options: SearchOptions) -> SearchResult {
        let search = Self::new(testcase, oracle, options);
        
        let mut input = testcase.clone();

        search.log(&format!("Starting search: {:?}", input));

        let start = std::time::Instant::now();

        search.find_relations(&mut input);

        let total_test_ms = start.elapsed().as_millis() as u64;
        
        let test_count = *search.test_count.borrow();
        let target_test_ms = *search.target_test_ms.borrow();

        let found_any = input.relations.len() > 0;

        SearchResult {
            input,
            test_count,
            target_test_ms,
            total_test_ms,
            found_any
        }
    }

    /// Performs multiple-passes over the input searching for relations.
    /// 
    /// Invokes `find_relations_inner` in a loop until no more relations are found or the max number of iterations is reached.
    /// 
    /// Returns true if any relations were found.
    fn find_relations(&self, input: &mut Structured) {
        self.log("Starting search...");

        let start = std::time::Instant::now();

        let mut iter = 0;
        while iter < self.options.max_iters {
            iter += 1;
            self.log(&format!("Iteration {}", iter));

            let found = self.find_relations_inner(input);
            if !found {
                // Exit if no relations were found this iteration.
                break;
            }
        }

        let elapsed = start.elapsed().as_millis() as u64;

        self.log(&format!("Search completed (total: {} ms) (target: {} ms)", elapsed, *self.target_test_ms.borrow()));
    }

    /// Performs a single-pass over the input searching for relations.
    /// 
    /// Returns true if any relations were found.
    fn find_relations_inner(&self, input: &mut Structured) -> bool
    where 
        O: FnMut(&[u8]) -> &'o [u8]
    {
        // Efficiency
        input.raw.reserve(0x100);
        let mut lost_indices = Vec::with_capacity(self.focus_indices.len()); // Maximum possible loss.
        let mut anchor_visited_cache: Vec<u8> = vec![0; input.raw.len()];
        let mut test_buffer = input.get_raw().to_vec();
        test_buffer.reserve(0x100);

        let mut blocked_points = vec![0; input.raw.len()];
        for rel in input.relations.iter() {
            for i in rel.pos..rel.pos + rel.size {
                blocked_points[i] = 1;
            }
        }

        let mut found = false;

        let seed_data = input.get_raw().to_vec();

        let mut inflection_points = input.inflection_points();

        let rel_types = vec![
            (8, true), (8, false),
            (4, true), (4, false),
            (2, true), (2, false),
            (1, true),
        ];

        // Iterate over field placement.
        for i in 0..seed_data.len() {
            'inner: for (size, le) in rel_types.iter() {
                if i + size > seed_data.len() {
                    continue 'inner;
                }

                let curr_size: usize = match (size, le) {
                    (2, false) => u16::from_be_bytes([seed_data[i], seed_data[i+1]]).into(),
                    (4, false) => u32::from_be_bytes([seed_data[i], seed_data[i+1], seed_data[i+2], seed_data[i+3]]) as usize,
                    (8, false) => u64::from_be_bytes([seed_data[i], seed_data[i+1], seed_data[i+2], seed_data[i+3], seed_data[i+4], seed_data[i+5], seed_data[i+6], seed_data[i+7]]) as usize,
                    (1, true) => u8::from_le_bytes([seed_data[i]]).into(),
                    (2, true) => u16::from_le_bytes([seed_data[i], seed_data[i+1]]).into(),
                    (4, true) => u32::from_le_bytes([seed_data[i], seed_data[i+1], seed_data[i+2], seed_data[i+3]]) as usize,
                    (8, true) => u64::from_le_bytes([seed_data[i], seed_data[i+1], seed_data[i+2], seed_data[i+3], seed_data[i+4], seed_data[i+5], seed_data[i+6], seed_data[i+7]]) as usize,
                    _ => panic!("Unsupported size")
                };
        
                // Does this look like a size/offset field?
                if curr_size == 0 || curr_size > seed_data.len() as usize {
                    continue 'inner;
                }

                let shift_amount = if size == &1 {
                    let max_shift = 0xff - curr_size;
                    if max_shift == 0 {
                        continue 'inner;
                    }
                    0x20.min(max_shift)
                } else {
                    // Shift by 0xff so we overflow the first byte in most cases.
                    // This helps to differentiate between little and big endian.
                    0xff
                };

                // Check if the field is blocked.
                for k in 0..*size {
                    if blocked_points[i+k] != 0 {
                        continue 'inner;
                    }
                }

                let mut potential = Relation {
                    pos: i,
                    value: curr_size as u64,
                    size: *size,
                    le: *le,
                    anchor: usize::MAX,
                    insert: usize::MAX,
                    enabled: true,
                    old_pos: 0,
                    old_anchor: 0,
                    old_insert: 0,
                    old_value: 0,
                };

                // Backup current state.
                input.save_relations();

                // Corrupt the field and measure lost features.
                potential.value = (curr_size as u64) + (shift_amount as u64);
                potential.apply(&mut test_buffer);

                lost_indices.clear();
                let ft = self.test(&test_buffer);
                for idx in self.focus_indices.iter() {
                    if ft[*idx] == 0 {
                        lost_indices.push(*idx);
                    }
                }

                if self.options.extra_verbose {
                    println!("Testing relation (size={}, le={}, pos={}, value={})", size, le, i, curr_size);
                    self.print_buffer(&test_buffer);
                    println!("lost: {:?} -- thresh: {:?}", lost_indices.len(), self.loss_threshold);
                }

                // Restore the original buffer.
                test_buffer[i..i+size].copy_from_slice(&seed_data[i..i+size]);

                if lost_indices.len() < self.loss_threshold {
                    continue 'inner;
                }

                // Iterate over inflection points and try to find a suitable anchor/insertion:
                anchor_visited_cache.fill(0);
                
                let mut curr_recover = self.options.recover_threshold;

                match size {
                    1 => {
                        self.check_anchor(input, i, i+size, shift_amount, &mut test_buffer, &seed_data, &mut lost_indices, &mut curr_recover, &mut potential, &mut anchor_visited_cache, &mut blocked_points);
                    }
                    2 => {
                        self.check_anchor(input, i, 0, shift_amount, &mut test_buffer, &seed_data, &mut lost_indices, &mut curr_recover, &mut potential, &mut anchor_visited_cache, &mut blocked_points);
                        self.check_anchor(input, i, i, shift_amount, &mut test_buffer, &seed_data, &mut lost_indices, &mut curr_recover, &mut potential, &mut anchor_visited_cache, &mut blocked_points);
                        self.check_anchor(input, i, i+size, shift_amount, &mut test_buffer, &seed_data, &mut lost_indices, &mut curr_recover, &mut potential, &mut anchor_visited_cache, &mut blocked_points);
                    }
                    _ => {
                        // Check local inflection points first.
                        self.check_anchor(input, i, i+size+7, shift_amount, &mut test_buffer, &seed_data, &mut lost_indices, &mut curr_recover, &mut potential, &mut anchor_visited_cache, &mut blocked_points);
                        self.check_anchor(input, i, i+size+6, shift_amount, &mut test_buffer, &seed_data, &mut lost_indices, &mut curr_recover, &mut potential, &mut anchor_visited_cache, &mut blocked_points);
                        self.check_anchor(input, i, i+size+5, shift_amount, &mut test_buffer, &seed_data, &mut lost_indices, &mut curr_recover, &mut potential, &mut anchor_visited_cache, &mut blocked_points);
                        self.check_anchor(input, i, i+size+4, shift_amount, &mut test_buffer, &seed_data, &mut lost_indices, &mut curr_recover, &mut potential, &mut anchor_visited_cache, &mut blocked_points);
                        self.check_anchor(input, i, i+size+3, shift_amount, &mut test_buffer, &seed_data, &mut lost_indices, &mut curr_recover, &mut potential, &mut anchor_visited_cache, &mut blocked_points);
                        self.check_anchor(input, i, i+size+2, shift_amount, &mut test_buffer, &seed_data, &mut lost_indices, &mut curr_recover, &mut potential, &mut anchor_visited_cache, &mut blocked_points);
                        self.check_anchor(input, i, i+size+1, shift_amount, &mut test_buffer, &seed_data, &mut lost_indices, &mut curr_recover, &mut potential, &mut anchor_visited_cache, &mut blocked_points);
                        self.check_anchor(input, i, 0, shift_amount, &mut test_buffer, &seed_data, &mut lost_indices, &mut curr_recover, &mut potential, &mut anchor_visited_cache, &mut blocked_points);
                        self.check_anchor(input, i, i, shift_amount, &mut test_buffer, &seed_data, &mut lost_indices, &mut curr_recover, &mut potential, &mut anchor_visited_cache, &mut blocked_points);
                        self.check_anchor(input, i, i+size, shift_amount, &mut test_buffer, &seed_data, &mut lost_indices, &mut curr_recover, &mut potential, &mut anchor_visited_cache, &mut blocked_points);
                    
                        // If we found a match here, bail early, otherwise search the rest of the inflection points.
                        if potential.insert == usize::MAX {
                            for anchor in inflection_points.iter() {
                                self.check_anchor(input, i, *anchor, shift_amount, &mut test_buffer, &seed_data, &mut lost_indices, &mut curr_recover, &mut potential, &mut anchor_visited_cache, &mut blocked_points);
                            }
                        }
                    }
                }

                if potential.insert == usize::MAX {
                    // No valid insertion point found.
                    continue 'inner;
                }

                // Reset and update the structure.
                potential.value = curr_size as u64;
                self.log_child("REL", &format!("found REL field at {} (size: {}, le: {}, anchor: {}, insert: {}, value: {})", i, size, le, potential.anchor, potential.insert, potential.value));
                input.add_relation(potential);

                // Update the field.
                inflection_points = input.inflection_points();
                
                // Update the blocked points.
                for k in 0..*size {
                    blocked_points[i+k] = 1;
                }
                
                found = true;
            }
        }

        found
    }

    #[inline]
    fn check_anchor(&self, input: &mut Structured, field_pos: usize, anchor: usize, shift_amount: usize, test_buffer: &mut Vec<u8>, seed_data: &[u8], lost_indices: &mut Vec<usize>, curr_recover: &mut f64, potential: &mut Relation, anchor_visited_cache: &mut Vec<u8>, blocked_points: &mut Vec<u8>) {
        let ins = anchor + potential.value as usize - shift_amount;
        
        // Out of bounds (insertion).
        if ins > seed_data.len() {
            return;
        }

        // Anchor already visited.
        if anchor >= seed_data.len() || anchor_visited_cache[anchor] != 0 {
            return;
        }
        anchor_visited_cache[anchor] = 1;

        if self.options.extra_verbose {
            self.log_child("REL", &format!("Testing insertion at {} (anchor: {}, shift: {})", ins, anchor, shift_amount));
        }

        if input.on_insert(ins, shift_amount).is_err() {
            // Error happens before buffer resizing, but we need to fix relation state.
            input.restore_relations();
            return;
        }

        // Update the buffer.
        test_buffer.resize(seed_data.len() + shift_amount, 0);

        test_buffer[ins+shift_amount..].copy_from_slice(&seed_data[ins..]); // Copy the shifted data.
        test_buffer[ins..ins+shift_amount].fill(0x41); // Fill the gap with 0x41.

        // Update the relation.
        {
            if ins < field_pos { potential.pos += shift_amount; }
            potential.apply(test_buffer);
            potential.pos = field_pos;
        }
        input.sanitize_buffer(test_buffer);

        if self.options.extra_verbose {
            self.print_buffer(&test_buffer);
        }

        let ft = self.test(&test_buffer);

        // Restore the original state.
        input.restore_relations();

        // Restore the original buffer
        test_buffer.resize(seed_data.len(), 0);
        test_buffer.copy_from_slice(&seed_data);

        let mut recovered = 0;
        for idx in lost_indices.iter() {
            if ft[*idx] != 0 {
                recovered += 1;
            }
        }
        let recovered_ratio = recovered as f64 / lost_indices.len() as f64;

        if self.options.extra_verbose {
            println!("Recovered: {:?} ({}%)", recovered, recovered_ratio * 100.0);
        }

        if recovered_ratio >= *curr_recover {
            // Valid insertion point.
            potential.insert = ins;
            potential.anchor = anchor;
            *curr_recover = recovered_ratio;
        }
    }

    fn log(&self, msg: &str) {
        if self.options.verbose {
            println!("[{}] (#{}) {}", "SEARCH".cyan(), self.test_count.borrow(), msg);
        }
    }

    fn log_child(&self, sub: &str, msg: &str) {
        if self.options.verbose {
            println!("[{}][{}] (#{}) {}", "SEARCH".cyan(), sub.purple(), self.test_count.borrow(), msg);
        }
    }

    fn print_buffer(&self, buffer: &[u8]) {
        for i in 0..buffer.len() {
            if i % 16 == 0 {
                print!("\n");
            }
            print!("{:02x} ", buffer[i]);
        }
        print!("\n");
    }

    fn test(&self, data: &[u8]) -> &'o [u8] {
        *self.test_count.borrow_mut() += 1;
        let start = std::time::Instant::now();
        let res = (self.oracle.borrow_mut())(data);
        let elapsed = start.elapsed().as_millis();
        *self.target_test_ms.borrow_mut() += elapsed as u64;
        res
    }
}
