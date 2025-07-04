
FROM ubuntu:22.04
RUN apt-get update && apt-get install -y make autoconf automake libtool curl \
  cmake llvm-dev libclang-dev clang \
  libgmp-dev pkg-config git

# Install a newer version of OCaml than provided by Ubuntu 16.04 (base version for this image)
# RUN curl -sL https://raw.githubusercontent.com/ocaml/opam/master/shell/install.sh -o install.sh && \
#   echo | sh install.sh && \
#   opam init --disable-sandboxing --yes --compiler=4.11.2 && \
#   opam install ocamlbuild ocamlfind --yes && \
#   CFLAGS= opam install zarith --yes

RUN git clone --depth 1 https://github.com/bytecodealliance/wasm-tools /fuzz/wasm-tools

RUN git clone --depth 1 https://github.com/bytecodealliance/regalloc2 /fuzz/regalloc2

RUN git clone --depth 1 https://github.com/bytecodealliance/wasmtime /fuzz/wasmtime
# WORKDIR wasmtime
#RUN git submodule update --init --recursive

#RUN git clone --depth 1 https://github.com/bytecodealliance/wasmtime-libfuzzer-corpus wasmtime-libfuzzer-corpus

COPY build.sh /fuzz

FROM structor_base_lib
COPY --from=0 /fuzz /fuzz

RUN apt-get install -y jq unzip libgmp-dev

RUN curl -sL https://raw.githubusercontent.com/ocaml/opam/master/shell/install.sh -o install.sh && \
  echo | sh install.sh && \
  opam init --disable-sandboxing --yes --compiler=4.11.2 && \
  opam install ocamlbuild ocamlfind --yes && \
  CFLAGS= opam install zarith --yes

RUN cargo install cargo-fuzz && \
    rustup default nightly

WORKDIR /fuzz
