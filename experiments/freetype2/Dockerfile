FROM frameshift_base

RUN apt-get update &&  \
    apt-get upgrade -y && \
    apt-get install -y \
    make \
    autoconf \
    libtool \
    wget

WORKDIR /fuzz

RUN git clone git://git.sv.nongnu.org/freetype/freetype2.git && \
    cd freetype2 && \
    git checkout cd02d359a6d0455e9d16b87bf9665961c4699538
RUN git clone https://github.com/unicode-org/text-rendering-tests.git TRT
RUN wget https://github.com/libarchive/libarchive/releases/download/v3.4.3/libarchive-3.4.3.tar.xz

ENV SRC=/fuzz
ENV OUT=/out
ENV WORK=/work
ENV CC=frameshift_afl_cc
ENV CXX=frameshift_afl_cxx
ENV CCFLAGS=""
ENV CXXFLAGS=""

RUN mkdir -p $OUT

WORKDIR /fuzz/freetype2
COPY build.sh .

RUN ./build.sh
