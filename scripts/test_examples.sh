#!/bin/sh
set -eu

mkdir -p build
cargo build --quiet --manifest-path emulator/Cargo.toml

for program in hello arithmetic branch call; do
    python3 assembler/assembler.py "examples/$program.s" \
        -o "build/$program.bin" --mem "build/$program.mem"
    emulator/target/debug/pynq-cpu-emulator "build/$program.bin" \
        --max-steps 10000 > "build/$program.out"
done

check_output() {
    program=$1
    expected=$2
    actual=$(cat "build/$program.out")
    if [ "$actual" != "$expected" ]; then
        echo "$program の出力が不一致です: '$actual'（期待値: '$expected'）" >&2
        exit 1
    fi
}

check_output hello "Hello, PYNQ CPU!"
check_output arithmetic "OK"
check_output branch "0123456789"
check_output call "CALL OK"
echo "エミュレータの全サンプルテスト成功"
