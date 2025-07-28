FROM frameshift_base_py

WORKDIR /fuzz
ENV SRC /fuzz

RUN apt-get update && apt-get install -y make autoconf automake libtool
RUN git clone https://github.com/etingof/pyasn1 /pyasn1 && \
    git -C /pyasn1 checkout db8f1a7 && \
    cd /pyasn1 && \
    python3 -m pip install .

COPY fuzz_decode.py seed.asn $SRC/
RUN mkdir -p input output
