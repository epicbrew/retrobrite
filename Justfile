
unittest:
    cargo test

nestest:
    -RUST_LOG=debug cargo run -- --pc 49152 -c 26555 nestest/nestest.nes > out.log 2>&1
    sed -rie 's/^\[.*\] //g' out.log

test ROM:
    RUST_LOG=debug cargo run -- "{{ROM}}"

