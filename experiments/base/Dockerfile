
FROM ubuntu:22.04
ENV TERM xterm-256color

RUN apt-get update && \
    apt-get -y upgrade && \
    apt-get -y install \
    clang make cmake git wget curl tar gdb vim \
    python3.11 python3-pip \
    pkg-config libssl-dev

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

ENV PATH="/root/.cargo/bin:${PATH}"

COPY rust-toolchain.toml /src/rust-toolchain.toml
COPY /frameshift_afl /src/frameshift_afl
COPY /third_party /src/third_party

ENV LIBAFL_EDGES_MAP_SIZE=400000

RUN --mount=type=cache,target=/root/.cargo/registry \
    --mount=type=cache,target=/src/frameshift_afl/target \
    --mount=type=cache,target=/src/third_party/target \
    --mount=type=cache,target=/root/.rustup \
    cd /src/frameshift_afl && \
    OUT_DIR=/deps cargo build --release && \
    cargo install --path . && \
    cp target/release/libframeshift_afl.a /root/.cargo/bin/libframeshift_afl.a && \
    mkdir -p /src/tmp && \
    cp -r target/release/build/libafl_cc-* /src/tmp

RUN cd /src/frameshift_afl && \
    mkdir -p target/release/build && \
    cp -r /src/tmp/libafl_cc-* target/release/build/

RUN cd /src/third_party/LibAFL/fuzzers/fuzzbench && \
    clang -c stub_rt.c && \
    ar r /stub_rt.a stub_rt.o
