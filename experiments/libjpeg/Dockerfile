
FROM ubuntu:22.04

ENV SRC=/fuzz

RUN apt-get update && \
    apt-get install -y make yasm cmake git zip

WORKDIR $SRC

RUN git clone https://github.com/libjpeg-turbo/fuzz && \
    cat fuzz/branches.txt | while read branch; do \
      git clone https://github.com/libjpeg-turbo/libjpeg-turbo \
          -b $branch libjpeg-turbo.$branch; \
    done

RUN git clone https://github.com/libjpeg-turbo/seed-corpora && \
    git -C seed-corpora checkout 7c9ea5ffaac76ef618657978c9fdfa845d310b93
RUN cd seed-corpora && \
    zip -r ../decompress_fuzzer_seed_corpus.zip \
        afl-testcases/jpeg* \
        bugs/decompress* \
        $SRC/libjpeg-turbo/testimages/*.jpg
RUN cd seed-corpora && \
    zip -r ../compress_fuzzer_seed_corpus.zip \
        afl-testcases/bmp \
        afl-testcases/gif* \
        bugs/compress* \
        $SRC/libjpeg-turbo/testimages/*.bmp \
        $SRC/libjpeg-turbo/testimages/*.ppm
RUN rm -rf seed-corpora

RUN mkdir -p /fuzz
FROM frameshift_base
COPY --from=0 /fuzz /fuzz

ENV SRC=/fuzz
ENV OUT=/out
ENV CC=frameshift_afl_cc
ENV CXX=frameshift_afl_cxx
ENV CCFLAGS=""
ENV CXXFLAGS=""
ENV WORK="/out"
ENV SANITIZER=""
ENV LIB_FUZZING_ENGINE="/stub_rt.a"

RUN mkdir -p $OUT

COPY build.sh $SRC/

RUN cd $SRC && ./build.sh

COPY seed.jpg /out/seed.jpg

WORKDIR /out