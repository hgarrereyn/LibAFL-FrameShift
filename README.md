# LibAFL FrameShift Prototype

## Overview

This repository contains an implementation of FrameShift for LibAFL.

The implementation is split into several parts:

- [frameshift_afl/src/core](frameshift_afl/src/core) contains the core algorithms:
    - [search.rs](frameshift_afl/src/core/search.rs): the double-mutant search algorithm.
    - [structured.rs](frameshift_afl/src/core/structured.rs): code for tracking and re-serializing relation fields during mutation.
- [frameshift_afl/src/components](frameshift_afl/src/components) contains the LibAFL-specific components:
    - [gen.rs](frameshift_afl/src/components/gen.rs): a simple generator that generates a fixed input.
    - [search_metadata.rs](frameshift_afl/src/components/search_metadata.rs): metadata for the search stage.
    - [search_stage.rs](frameshift_afl/src/components/search_stage.rs): the search stage (runs once on every new input).
    - [structured_input.rs](frameshift_afl/src/components/structured_input.rs): the new structured input type.
    - [wrapped_mutator.rs](frameshift_afl/src/components/wrapped_mutator.rs): describes a mutator which wraps another arbitrary `BytesInput` mutator, while applying structure-aware mutations.
- [frameshift_afl/src/bin](frameshift_afl/src/bin) contains the LibAFL compiler wrappers (`frameshift_afl_cc` and `frameshift_afl_cxx`).

There are two reference fuzzer implementations:
- [frameshift_afl/src/fuzz_afl.rs](frameshift_afl/src/fuzz_afl.rs): A direct baseline LibAFL fuzzer using the fuzzbench configuration.
- [frameshift_afl/src/fuzz_frameshift.rs](frameshift_afl/src/fuzz_frameshift.rs): The FrameShift-enabled version of the fuzzer which uses the `StructuredInput` type and runs the `SearchStage` on every input.


## Usage

When you build the fuzzer (with `frameshift_afl_cc` or `frameshift_afl_cxx`), general usage is the same as the fuzzbench version:

`./fuzzer -i <input_dir> -o <output_dir> --timeout <timeout> [--tokens <tokenfile>] [--logfile <logfile>]`

By default, this will run in FrameShift mode. The following additional options are available:

- `--disable-frameshift`: Run in the baseline LibAFL mode.
- `--verbose-search`: Print information about the search process.
- `--verbose-search-extra`: Print even more information about the search process.
- `--search-max-iters <n>`: The maximum number of iterations to run the search for (default: 100).
- `--search-loss-threshold <n>`: The loss threshold for the search (default: 0.05).
- `--search-recover-threshold <n>`: The recover threshold for the search (default: 0.2).

## Library Usage

There is also a simple library interface in [frameshift_afl_lib](frameshift_afl_lib/src/lib.rs) which describes how to use LibAFL as a drop in replacement for libFuzzer backends (e.g. for use with Atheris or cargo-fuzz).

## Experiments

Several Dockerized experiments are provided in the [experiments](experiments) directory to demonstrate how to use FrameShift in various modes. To build and run an experiment, run `./run <experiment_name>` in the experiment directory. This will build the docker image and give you a shell in the container to run the fuzzer.

E.g.
```bash
cd experiments
./run libpng

# (inside the container:)
./libpng_read_fuzzer -i input/ -o output/ --verbose-search
```

There is also an example seed file provided for each experiment, you can analyze it by running `<target> -a <seed_file>`.

#### Available experiments

| Experiment | Language | Target |
| ---------- | -------- | ------ |
| libpng | C | `./libpng_read_fuzzer ...` |
| bloaty | C | `./fuzz_target ...` |
| image-png | Rust | `./decode ...` |
| pyasn1 | Python | `python3 fuzz_asn1.py ...` |
