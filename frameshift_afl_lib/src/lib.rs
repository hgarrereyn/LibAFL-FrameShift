
use std::os::raw::{c_char, c_int};

use clap::Parser;
use libafl::observers::StdMapObserver;
use libafl_targets::extra_counters;
use frameshift_afl::{entrypoint, Cli};

#[no_mangle]
#[allow(non_snake_case)]
pub extern "C" fn LLVMFuzzerRunDriver(
    _argc: *const c_int,
    _argv: *const *const c_char,
    harness_fn: Option<extern "C" fn(*const u8, usize) -> c_int>,
) {
    assert!(harness_fn.is_some(), "No harness callback provided");
    let harness_fn = harness_fn.unwrap();

    // Ensure we see some coverage before starting fuzzing
    let dummy = b"initial";
    harness_fn(dummy.as_ptr(), dummy.len());

    let res = Cli::parse();

    let mut fuzz_fn = |data: &[u8]| -> i32 {
        harness_fn(data.as_ptr(), data.len() as usize)
    };

    let edges = unsafe { extra_counters() };
    let obs = StdMapObserver::from_mut_slice(
        "edges",
        edges.into_iter().next().unwrap(),
    );
    
    entrypoint(res.options, &mut fuzz_fn, obs);
}
