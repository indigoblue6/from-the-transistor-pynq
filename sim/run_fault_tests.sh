#!/bin/sh
set -eu

sources="rtl/alu.sv rtl/register_file.sv rtl/decoder.sv rtl/cpu.sv rtl/memory_map.sv sim/cpu_fault_tb.sv"
mkdir -p build
if command -v iverilog >/dev/null 2>&1; then
    iverilog -g2012 -s cpu_fault_tb -o build/cpu_fault_tb.vvp $sources
    vvp build/cpu_fault_tb.vvp
elif command -v verilator >/dev/null 2>&1; then
    verilator --binary --timing -Wno-fatal --top-module cpu_fault_tb \
        --Mdir build/obj_fault -o cpu_fault_tb $sources
    build/obj_fault/cpu_fault_tb
else
    echo "faultテストを実行できるRTLシミュレータがありません。" >&2
    exit 2
fi
