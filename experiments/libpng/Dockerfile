FROM ubuntu:22.04

RUN apt-get update && \
    apt-get install -y git

WORKDIR /fuzz

RUN git clone --depth 1 https://github.com/madler/zlib.git
RUN git clone --depth 1 https://github.com/pnggroup/libpng.git

FROM frameshift_base
COPY --from=0 /fuzz /fuzz

RUN apt-get install -y make autoconf automake libtool zlib1g-dev zip

ENV SRC=/fuzz
ENV OUT=/out
ENV CC=frameshift_afl_cc
ENV CXX=frameshift_afl_cxx
ENV CCFLAGS=""
ENV CXXFLAGS=""

RUN mkdir -p $OUT

WORKDIR /fuzz/libpng
COPY build.sh .

RUN ./build.sh

COPY seed.png /out/seed.png
WORKDIR /out

RUN mkdir input output && \
    cp seed.png input/
