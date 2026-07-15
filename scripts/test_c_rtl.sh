#!/bin/sh
set -eu

programs="hello arithmetic function recursion pointer array fizzbuzz"
mkdir -p build
for name in $programs; do
    cargo run --quiet --manifest-path compiler/Cargo.toml -- \
        "examples/c/$name.pc" -o "build/c_$name.s"
    python3 assembler/assembler.py "build/c_$name.s" \
        -o "build/c_$name.bin" --mem "build/c_$name.mem"
    cargo run --quiet --manifest-path emulator/Cargo.toml -- \
        "build/c_$name.bin" --max-steps 5000000 --dump-registers > "build/c_$name.emulator" 2> "build/c_$name.emulator-registers"
    OUTPUT_FILE="build/c_$name.rtl" REGISTER_FILE="build/c_$name.rtl-registers" MAX_CYCLES=5000000 \
        sh sim/run_sim.sh "c_$name" > "build/c_$name.sim.log"
    if ! cmp -s "build/c_$name.emulator" "build/c_$name.rtl"; then
        echo "$nameのエミュレータとRTL出力が一致しません" >&2
        diff -u "build/c_$name.emulator" "build/c_$name.rtl" >&2 || true
        exit 1
    fi
    python3 scripts/compare_registers.py "build/c_$name.emulator-registers" "build/c_$name.rtl-registers"
    echo "$name: 差分テスト成功"
done

echo "PynqC RTL差分テスト成功"
