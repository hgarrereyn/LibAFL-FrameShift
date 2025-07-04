FROM ubuntu:22.04

RUN apt-get update && \
    apt-get -y upgrade && \
    apt-get -y install \
    python3 python3-pip git

RUN pip3 install --upgrade pip brotli
RUN git clone https://github.com/behdad/fonttools /fuzz/fonttools
COPY *py /fuzz/

FROM structor_base_lib
COPY --from=0 /fuzz /fuzz

RUN apt-get update && \
    apt-get -y install \
    clang

RUN cd /fuzz/fonttools/ && \
    python3 -m pip install .

ENV LIBFUZZER_LIB /libstructor_afl_lib.a
RUN git clone --depth 1 https://github.com/google/atheris.git
RUN cd atheris && \
    pip install .

WORKDIR /fuzz

RUN mkdir input output
