FROM frameshift_base_rust

RUN mkdir /fuzz
RUN git clone https://github.com/image-rs/image-png /fuzz/image-png
WORKDIR /fuzz

COPY build.sh /fuzz/build.sh
COPY seed.png /fuzz/seed.png

RUN cd /fuzz && \
    mkdir input output && \
    cp seed.png input/ && \
    ./build.sh
