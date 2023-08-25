
unittest:
    cargo test

nestest:
    # Run retrobrite
    -RUST_LOG=debug cargo run -- --pc 49152 -c 26555 nestest/nestest.nes > retrobrite.log 2>&1
    # Remove logging lines/formatting not included in nestest log
    sed -re 's/^\[.*\] //g' retrobrite.log | grep -e "^[0-9A-F]\{4\}" > retrobrite-nestest.log
    # Check if our log matches with the nestest "golden" log
    diff -u retrobrite-nestest.log nestest/nestest-retrobrite-formatted.log

run ROM:
    RUST_LOG=debug cargo run -- "{{ROM}}"

