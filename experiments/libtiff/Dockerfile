# Copyright 2018 Google Inc.
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#      http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.
#
################################################################################

FROM ubuntu:22.04
ENV SRC=/fuzz
WORKDIR $SRC
RUN apt-get update && apt-get install -y make autoconf automake libtool cmake nasm git
RUN git clone --depth 1 https://gitlab.com/libtiff/libtiff
RUN git clone --depth 1 https://github.com/madler/zlib
RUN git clone --depth 1 https://github.com/libjpeg-turbo/libjpeg-turbo
RUN git clone https://www.cl.cam.ac.uk/~mgk25/git/jbigkit
ADD https://lcamtuf.coredump.cx/afl/demo/afl_testcases.tgz afl_testcases.tgz
ADD https://raw.githubusercontent.com/google/AFL/debe27037b9444bbf090a0ffbd5d24889bb887ae/dictionaries/tiff.dict tiff.dict
WORKDIR libtiff
COPY build.sh $SRC/libtiff

FROM structor_base
COPY --from=0 /fuzz /fuzz    

RUN apt-get install -y make autoconf automake libtool cmake nasm zip

ENV SRC=/fuzz
ENV OUT=/out
ENV CC=structor_afl_cc
ENV CXX=structor_afl_cxx
ENV CFLAGS=""
ENV CCFLAGS=""
ENV CXXFLAGS=""
ENV WORK="/out"
ENV SANITIZER=""
ENV LIB_FUZZING_ENGINE="/stub_rt.a"
ENV ARCHITECTURE="?"

RUN mkdir -p $OUT

RUN mkdir -p /opt/seeds
RUN cd $SRC/libtiff && ./build.sh

WORKDIR $OUT
COPY seeds /out/seeds

RUN mkdir -p input output && \
    cp /out/seeds/* input/