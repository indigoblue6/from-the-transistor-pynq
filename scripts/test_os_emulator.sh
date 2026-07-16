#!/bin/sh
set -eu

# 引数はMakefile上の試験分類を示す。現段階では同じ統合イメージで実機能を検証する。
case "${1:-}" in
    ""|--allocator-only|--ramfs-only|--shell-only) ;;
    *) echo "不明な試験モード: $1" >&2; exit 2 ;;
esac

sh scripts/build_os_image.sh
mkdir -p build
cargo run --quiet --manifest-path emulator/Cargo.toml -- \
    build/indigo-os.bin \
    --uart-input tests/input/shell.txt \
    --max-steps 10000000 >build/os-emulator.out

for expected in "IndigoOS 0.1" "help      show command list" "hello world" \
    "0    RUNNING    shell" "free=16" "Hello from IndigoOS!" \
    "unknown command: unknown" "reboot requested"; do
    grep -F "$expected" build/os-emulator.out >/dev/null || {
        echo "エミュレータOS出力に不足: $expected" >&2
        exit 1
    }
done

echo "エミュレータOS出力検証成功"
