#!/bin/sh
set -eu

mkdir -p build
cargo run --quiet --manifest-path compiler/Cargo.toml -- \
    kernel/indigo_os.pc -o build/indigo-kernel-generated.s
sed 's/^\.org 0$/.org 0x100/' build/indigo-kernel-generated.s > build/indigo-kernel-relocated.s
{
    sed -n '1,999p' kernel/arch/indigo32/boot.s
    sed -n '2,99999p' build/indigo-kernel-relocated.s
    sed -n '1,999p' kernel/arch/indigo32/exception.s
} > build/indigo-os.s
python3 assembler/assembler.py build/indigo-os.s \
    -o build/indigo-os.bin --mem build/indigo-os.mem
