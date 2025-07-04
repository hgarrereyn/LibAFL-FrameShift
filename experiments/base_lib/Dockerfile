
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
COPY /frameshift_afl_lib /src/frameshift_afl_lib
COPY /third_party /src/third_party

ENV LIBAFL_EDGES_MAP_SIZE=400000

RUN --mount=type=cache,target=/root/.cargo/registry \
    --mount=type=cache,target=/src/frameshift_afl_lib/target \
    --mount=type=cache,target=/src/third_party/target \
    --mount=type=cache,target=/root/.rustup \
    cd /src/frameshift_afl_lib && \
    OUT_DIR=/deps cargo build --release && \
    cp target/release/libframeshift_afl_lib.a /libframeshift_afl_lib.a && \
    mkdir -p /src/tmp && \
    cp -r target/release/build/libafl_cc-* /src/tmp

RUN cd /src/frameshift_afl_lib && \
    mkdir -p target/release/build && \
    cp -r /src/tmp/libafl_cc-* target/release/build/
