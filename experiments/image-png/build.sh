export CUSTOM_LIBFUZZER_PATH=/libframeshift_afl_lib.a

(cd ./image-png/fuzz && \
    cargo rustc --bin decode -- \
        -Cpasses=sancov-module \
        -Cllvm-args=-sanitizer-coverage-level=4 \
        -Cllvm-args=-sanitizer-coverage-inline-8bit-counters \
        -Cllvm-args=-sanitizer-coverage-pc-table \
        -Zsanitizer=address \
        -Clink-arg=-Wl,--allow-multiple-definition)

cp ./image-png/fuzz/target/debug/decode /fuzz/decode
