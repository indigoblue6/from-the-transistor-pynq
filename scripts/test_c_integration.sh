#!/bin/sh
set -eu

mkdir -p build

compile_and_run() {
    name=$1
    expected=$2
    cargo run --quiet --manifest-path compiler/Cargo.toml -- \
        "examples/c/$name.pc" -o "build/c_$name.s"
    python3 assembler/assembler.py "build/c_$name.s" \
        -o "build/c_$name.bin" --mem "build/c_$name.mem"
    cargo run --quiet --manifest-path emulator/Cargo.toml -- \
        "build/c_$name.bin" --max-steps 5000000 > "build/c_$name.actual"
    printf '%s' "$expected" > "build/c_$name.expected"
    if ! cmp -s "build/c_$name.expected" "build/c_$name.actual"; then
        echo "$nameの出力が一致しません" >&2
        diff -u "build/c_$name.expected" "build/c_$name.actual" >&2 || true
        exit 1
    fi
}

compile_and_run hello 'Hello from PynqC!
'
compile_and_run arithmetic '42
'
compile_and_run variables '2 42
'
compile_and_run condition 'COND OK
'
compile_and_run loop '0123456879
'
compile_and_run function 'FUNCTION OK
'
compile_and_run recursion '120
'
compile_and_run pointer '42
'
compile_and_run array '42
'
compile_and_run string 'PynqC string 12
'
compile_and_run global 'GLOBAL OK
42
'
compile_and_run features '34
'
compile_and_run runtime_math '-3 -1 -42
'

cargo run --quiet --manifest-path compiler/Cargo.toml -- \
    examples/c/fizzbuzz.pc -o build/c_fizzbuzz.s
python3 assembler/assembler.py build/c_fizzbuzz.s \
    -o build/c_fizzbuzz.bin --mem build/c_fizzbuzz.mem
cargo run --quiet --manifest-path emulator/Cargo.toml -- \
    build/c_fizzbuzz.bin --max-steps 5000000 > build/c_fizzbuzz.actual
python3 scripts/verify_fizzbuzz.py build/c_fizzbuzz.actual

# Milestone 1: 非0のmain戻り値がSIM_EXITへ届くことを診断値で確認する。
cargo run --quiet --manifest-path compiler/Cargo.toml -- \
    examples/c/exit42.pc -o build/c_exit42.s
python3 assembler/assembler.py build/c_exit42.s \
    -o build/c_exit42.bin --mem build/c_exit42.mem
if cargo run --quiet --manifest-path emulator/Cargo.toml -- \
    build/c_exit42.bin --max-steps 100000 >build/c_exit42.out 2>build/c_exit42.err; then
    echo "exit42が成功終了してしまいました" >&2
    exit 1
fi
grep -q 'SIM_EXIT失敗: 42' build/c_exit42.err

echo "PynqCエミュレータ統合テスト成功"
