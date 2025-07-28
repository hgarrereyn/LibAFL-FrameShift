//! A singlethreaded libfuzzer-like fuzzer that can auto-restart.
use components::search_stage::SearchStageArgs;
use libafl::prelude::{MapObserver, StdMapObserver};
use libafl_targets::{extra_counters, libfuzzer_initialize, libfuzzer_test_one_input, std_edges_map_observer};
use libafl_bolts::{AsIter, AsSlice};
use mimalloc::MiMalloc;
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

use core::{search::{SearchContext, SearchOptions}, structured::Structured};
use std::{
    collections::HashSet, env, fs::{self}, path::PathBuf, time::{Duration, Instant}
};

use clap::{Args, Parser};


pub mod core;
pub mod components;
pub mod fuzz_afl;
pub mod fuzz_frameshift;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Positional arguments that can appear before or after named arguments
    pub args: Vec<String>,

    #[command(flatten)]
    pub options: Options,
}

#[derive(Args)]
pub struct Options {
    #[arg(short, long)]
    pub out: Option<String>,

    #[arg(short, long)]
    pub input: Option<String>,

    #[arg(short, long)]
    pub analyze: Option<String>,

    // Something like start:end:<hexstring>
    #[arg(short, long)]
    pub mutate_splice: Option<String>,

    #[arg(short, long)]
    pub tokens: Option<String>,

    #[arg(short, long, default_value = "libafl.log")]
    pub logfile: String,

    #[arg(short, long, default_value = "1200")]
    pub timeout: String,

    #[arg(short, long, default_value_t = false)]
    pub disable_frameshift: bool,

    #[arg(short, long, default_value_t = false)]
    pub verbose_search: bool,

    #[arg(short, long, default_value_t = false)]
    pub verbose_search_extra: bool,

    #[arg(short, long, default_value_t = 100)]
    pub search_max_iters: usize,

    #[arg(short, long, default_value_t = 0.05)]
    pub search_loss_threshold: f64,

    #[arg(short, long, default_value_t = 0.2)]
    pub search_recover_threshold: f64,

    #[arg(short, long, default_value_t = 0)]
    pub stress_analyze: u32,

    #[arg(short, long, default_value_t = 0)]
    pub stress_mutate: u32,

    #[arg(short, long)]
    pub tpm_experiment: Option<String>,
}

/// The fuzzer main (as `no_mangle` C function)
#[no_mangle]
pub extern "C" fn libafl_main() {
    let res = Cli::parse();
    
    let edges = {
        #[cfg(feature = "use_counters")]
        {
            let edges = unsafe { extra_counters() };
            let obs = StdMapObserver::from_mut_slice(
                "edges",
                edges.into_iter().next().unwrap(),
            );
            obs
        }

        #[cfg(not(feature = "use_counters"))]
        {
            let edges = unsafe { std_edges_map_observer("edges") };
            edges
        }
    };

    let args: Vec<String> = env::args().collect();
    if libfuzzer_initialize(&args) == -1 {
        println!("Warning: LLVMFuzzerInitialize failed with -1");
    }

    entrypoint(res.options, &mut libfuzzer_test_one_input, edges);
}

pub fn entrypoint<F>(res: Options, fuzz_fn: &mut F, obs: StdMapObserver<u8,false>,) 
where 
    F: Fn(&[u8]) -> i32,
{
    if res.tpm_experiment.is_some() {
        tpm_experiment(res, fuzz_fn, obs);
    } else if res.analyze.is_some() {
        analyze(res, fuzz_fn, obs);
    } else if res.input.is_some() && res.out.is_some() {
        fuzz(res, fuzz_fn, obs);
    } else {
        println!("Must specify (input and output) or (analyze) options");
    }
}

pub fn fuzz<F>(res: Options, fuzz_fn: &mut F, obs: StdMapObserver<u8,false>,) 
where 
    F: Fn(&[u8]) -> i32,
{
    println!(
        "Workdir: {:?}",
        env::current_dir().unwrap().to_string_lossy().to_string()
    );

    // For fuzzbench, crashes and finds are inside the same `corpus` directory, in the "queue" and "crashes" subdir.
    let mut out_dir = PathBuf::from(res.out.unwrap());
    if fs::create_dir(&out_dir).is_err() {
        println!("Out dir at {:?} already exists.", &out_dir);
        if !out_dir.is_dir() {
            println!("Out dir at {:?} is not a valid directory!", &out_dir);
            return;
        }
    }
    let mut crashes = out_dir.clone();
    crashes.push("crashes");
    out_dir.push("queue");

    let in_dir = PathBuf::from(res.input.unwrap());
    if !in_dir.is_dir() {
        println!("In dir at {:?} is not a valid directory!", &in_dir);
        return;
    }

    let tokens = res.tokens.map(PathBuf::from);

    let logfile = PathBuf::from(res.logfile);

    let timeout = Duration::from_millis(
        res.timeout
            .parse()
            .expect("Could not parse timeout in milliseconds"),
    );

    let search_options = SearchOptions {
        verbose: res.verbose_search,
        extra_verbose: res.verbose_search_extra,
        max_iters: res.search_max_iters,
        loss_threshold: res.search_loss_threshold,
        recover_threshold: res.search_recover_threshold,
    };

    match !res.disable_frameshift {
        true => {
            println!("Frameshift enabled");
            let search_args = SearchStageArgs {
                options: search_options,
            };

            fuzz_frameshift::fuzz_frameshift(fuzz_fn, obs, out_dir, crashes, &in_dir, tokens, 
                &logfile, timeout, search_args)
                .expect("An error occurred while fuzzing");
        }
        false => {
            println!("Frameshift disabled");
            fuzz_afl::fuzz_afl(fuzz_fn, obs, out_dir, crashes, &in_dir, tokens, &logfile, timeout)
                .expect("An error occurred while fuzzing");
        }
    }
}

pub fn analyze<F>(res: Options, fuzz_fn: &mut F, mut obs: StdMapObserver<u8,false>,) 
where 
    F: Fn(&[u8]) -> i32,
{
    let path = PathBuf::from(res.analyze.unwrap());
    println!("Analyzing {:?}", path);

    let raw = fs::read(path).expect("Could not read testcase");

    // Setup base.
    obs.reset_map().unwrap();
    fuzz_fn(&[]);

    let search_options = SearchOptions {
        verbose: res.verbose_search,
        extra_verbose: res.verbose_search_extra,
        max_iters: res.search_max_iters,
        loss_threshold: res.search_loss_threshold,
        recover_threshold: res.search_recover_threshold,
    };

    if res.stress_analyze > 0 {
        let start_time = Instant::now();

        let mut total_tests = 0;
        let mut target_ms = 0;
        let mut total_ms = 0;

        for _ in 0..res.stress_analyze {
            let mut oracle = |input: &[u8]| {
                {
                    obs.reset_map().unwrap();
                }
                fuzz_fn(input);
                let obs = obs.as_ref();
        
                // Convert to static lifetime - this is unsafe but needed for the oracle
                let slice = obs.as_slice();
                unsafe { std::mem::transmute::<&[u8], &'static [u8]>(slice) }
            };

            let testcase = Structured::raw(raw.clone());
            let search_res = SearchContext::search(&testcase, &mut oracle, search_options.clone());
            total_tests += search_res.test_count;
            target_ms += search_res.target_test_ms;
            total_ms += search_res.total_test_ms;
        }

        let end_time = Instant::now();
        let duration = end_time.duration_since(start_time);
        println!("Stress analyze time: {:?}", duration);
        println!("Total tests: {}", total_tests);
        println!("Target ms: {}", target_ms);
        println!("Total ms: {}", total_ms);
        println!("Efficiency: {}", (target_ms as f64 / total_ms as f64) * 100.0);
        return;
    }

    let mut oracle = |input: &[u8]| {
        {
            obs.reset_map().unwrap();
        }
        fuzz_fn(input);
        let obs = obs.as_ref();

        // Convert to static lifetime - this is unsafe but needed for the oracle
        let slice = obs.as_slice();
        unsafe { std::mem::transmute::<&[u8], &'static [u8]>(slice) }
    };

    let testcase = Structured::raw(raw);
    let search_res = SearchContext::search(&testcase, &mut oracle, search_options);
    println!("{:?}", search_res.input);

    if res.stress_mutate > 0 {
        let start_time = Instant::now();
        for _ in 0..res.stress_mutate {
            for idx in 0..search_res.input.get_raw().len() {
                let mut input = search_res.input.clone();
                input.insert(idx, &vec![0x41; 5]);
            }
        }
        let end_time = Instant::now();
        let duration = end_time.duration_since(start_time);
        println!("Stress mutate time: {:?}", duration);
    }
}


pub fn tpm_experiment<F>(res: Options, fuzz_fn: &mut F, mut obs: StdMapObserver<u8,false>,) 
where 
    F: Fn(&[u8]) -> i32,
{
    let path = PathBuf::from(res.tpm_experiment.unwrap());
    println!("TPM experiment {:?}", path);

    let raw = fs::read(path).expect("Could not read testcase");

    // Setup base.
    obs.reset_map().unwrap();
    fuzz_fn(&[]);

    let mut oracle = |input: &[u8]| {
        {
            obs.reset_map().unwrap();
        }
        fuzz_fn(input);
        let obs = obs.as_ref();

        let hit_indices = obs.iter().enumerate().filter(|(_, &v)| v != 0).map(|(i, _)| i).collect::<HashSet<_>>();
        hit_indices
    };

    let orig_coverage = oracle(&raw);
    println!("Original coverage: {:?}", orig_coverage.len());

    let shift_amt = 0x20;

    for i in 0..raw.len() {
        let mut input = raw.clone();
        input[i] += shift_amt;

        let coverage = oracle(&input);
        let shared_coverage = orig_coverage.intersection(&coverage).count();
        println!("IDX: {}, SHARED: {}", i, shared_coverage);

        for j in 0..=raw.len() {
            let mut insert_input = input.clone();
            insert_input.splice(j..j, vec![0x41; shift_amt as usize]);
            let coverage = oracle(&insert_input);
            let shared_coverage = orig_coverage.intersection(&coverage).count();
            println!("INSERT: {}:{}, SHARED: {}", i, j, shared_coverage);
        }

        for j in 0..=raw.len() {
            let mut insert_input = input.clone();
            insert_input[5] += shift_amt; // edit the commandsize
            insert_input.splice(j..j, vec![0x41; shift_amt as usize]);
            let coverage = oracle(&insert_input);
            let shared_coverage = orig_coverage.intersection(&coverage).count();
            println!("PROT_INSERT: {}:{}, SHARED: {}", i, j, shared_coverage);
        }
    }
}
