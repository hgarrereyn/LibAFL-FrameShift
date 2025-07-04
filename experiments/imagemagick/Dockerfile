FROM ubuntu:22.04

ENV SRC=/fuzz

RUN apt-get update && \
    apt-get install -y make autoconf automake groff libtool pkg-config cmake nasm autopoint
RUN git clone --depth 1 https://github.com/imagemagick/imagemagick
RUN git clone --depth 1 https://gitlab.com/libtiff/libtiff
RUN git clone --depth 1 https://github.com/strukturag/libde265
RUN git clone --depth 1 https://github.com/strukturag/libheif
RUN git clone --depth 1 https://github.com/uclouvain/openjpeg
RUN git clone --depth 1 https://github.com/webmproject/libwebp
RUN git clone --depth 1 https://github.com/madler/zlib
RUN git clone --depth 1 https://github.com/ebiggers/libdeflate
RUN git clone --depth 1 https://github.com/libjpeg-turbo/libjpeg-turbo
RUN git clone --depth 1 https://github.com/tukaani-project/xz
RUN git clone --depth 1 https://github.com/libraw/libraw
RUN git clone --depth 1 https://github.com/pnggroup/libpng
RUN git clone --depth 1 https://github.com/mm2/Little-CMS
RUN git clone --depth 1 https://github.com/freetype/freetype
RUN git clone --depth 1 https://gitlab.com/federicomenaquintero/bzip2.git
RUN git clone --depth 1 --recursive https://github.com/libjxl/libjxl
RUN git clone https://www.cl.cam.ac.uk/~mgk25/git/jbigkit # does not support shallow
ADD https://lcamtuf.coredump.cx/afl/demo/afl_testcases.tgz afl_testcases.tgz
COPY build.sh $SRC/
