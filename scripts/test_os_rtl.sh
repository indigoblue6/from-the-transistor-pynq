#!/bin/sh
set -eu

sh scripts/build_os_image.sh
mkdir -p build
sources="rtl/alu.sv rtl/register_file.sv rtl/decoder.sv rtl/timer.sv rtl/interrupt_controller.sv rtl/csr_file.sv rtl/cpu_indigo.sv rtl/memory_map_indigo.sv rtl/uart_rx.sv sim/os_tb.sv"
verilator --binary --timing -Wno-fatal --top-module os_tb \
    --Mdir build/obj_os -o os_tb $sources
build/obj_os/os_tb +MEM=build/indigo-os.mem +INPUT=tests/input/shell.txt +OUTPUT=build/os-rtl.out
for expected in "IndigoOS 0.1" "help      show command list" "hello world" \
    "0    RUNNING    shell" "free=16" "Hello from IndigoOS!" \
    "unknown command: unknown" "reboot requested"; do
    grep -F "$expected" build/os-rtl.out >/dev/null || {
        echo "RTL OS出力に不足: $expected" >&2
        exit 1
    }
done
echo "RTL OS出力検証成功"
