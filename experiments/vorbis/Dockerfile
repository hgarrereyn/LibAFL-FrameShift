FROM ubuntu:22.04

ENV SRC=/fuzz
WORKDIR $SRC

RUN apt-get update && \
    apt-get install -y \
    make \
    automake \
    autoconf \
    libtool \
    wget git zip

RUN git clone https://github.com/xiph/ogg.git && \
    git -C ogg checkout db5c7a49ce7ebda47b15b78471e78fb7f2483e22
RUN git clone https://github.com/xiph/vorbis.git
RUN wget -qO $SRC/decode_fuzzer.cc \
    https://raw.githubusercontent.com/google/oss-fuzz/688aadaf44499ddada755562109e5ca5eb3c5662/projects/vorbis/decode_fuzzer.cc

FROM structor_base
COPY --from=0 /fuzz /fuzz    

RUN apt-get install -y make autoconf automake libtool zlib1g-dev zip

ENV SRC=/fuzz
ENV OUT=/out
ENV CC=structor_afl_cc
ENV CXX=structor_afl_cxx
ENV CCFLAGS=""
ENV CXXFLAGS=""
ENV WORK="/out"
ENV SANITIZER=""
ENV LIB_FUZZING_ENGINE="/stub_rt.a"

COPY build.sh $SRC/

RUN mkdir -p $OUT

RUN mkdir -p /opt/seeds
RUN cd $SRC && ./build.sh

COPY sound.ogg /out/seed.ogg

WORKDIR $OUT
