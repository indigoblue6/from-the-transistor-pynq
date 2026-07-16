#!/bin/sh
set -eu

sh scripts/test_os_emulator.sh
sh scripts/test_os_rtl.sh

# 現在は同じ決定的入力を与え、UART文字列のアーキテクチャ結果を完全比較する。
if ! cmp -s build/os-emulator.out build/os-rtl.out; then
    echo "エミュレータとRTLのUART出力が一致しません" >&2
    diff -u build/os-emulator.out build/os-rtl.out >&2 || true
    exit 1
fi

echo "IndigoOS差分テスト成功: UART出力一致"
