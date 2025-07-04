FROM ubuntu:22.04

ENV SRC=/fuzz
WORKDIR $SRC

RUN apt-get update && \
    apt-get install -y \
        cmake \
        ninja-build \
        g++ \
        libz-dev git

RUN git clone \
    https://github.com/google/bloaty.git

WORKDIR bloaty
COPY build.sh $SRC/bloaty

FROM frameshift_base
COPY --from=0 /fuzz /fuzz

RUN apt-get install -y cmake ninja-build g++ libz-dev zip

ENV SRC=/fuzz
ENV OUT=/out
ENV CC=frameshift_afl_cc
ENV CXX=frameshift_afl_cxx
ENV CFLAGS=""
ENV CCFLAGS=""
ENV CXXFLAGS=""
ENV WORK="/work"
ENV SANITIZER=""
ENV LIB_FUZZING_ENGINE="/stub_rt.a"
ENV ARCHITECTURE="?"

RUN mkdir -p $OUT
RUN mkdir -p $WORK

RUN cd $SRC/bloaty && ./build.sh

WORKDIR $OUT
COPY seeds /out/seeds