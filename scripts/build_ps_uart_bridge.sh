#!/bin/sh
set -eu

# Vitis同梱のCortex-A9ツールチェーンで、OCM上で動くブリッジを生成する。
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
tool_root=${VITIS_GNU_ROOT:-/tools/AMD/2025.2/Vitis/gnu/aarch32/lin/gcc-arm-none-eabi/bin}
gcc="$tool_root/arm-none-eabi-gcc"
mkdir -p "$root/build/ps_uart_bridge"
"$gcc" -mcpu=cortex-a9 -marm -ffreestanding -fno-builtin -nostdlib \
    -Wl,-T,"$root/ps_uart_bridge/linker.ld" \
    "$root/ps_uart_bridge/startup.S" "$root/ps_uart_bridge/main.c" \
    -o "$root/build/ps_uart_bridge/bridge.elf"
