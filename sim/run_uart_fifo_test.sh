#!/bin/sh
set -eu
mkdir -p build
if command -v iverilog >/dev/null 2>&1; then
    iverilog -g2012 -s uart_fifo_tb -o build/uart_fifo_tb.vvp rtl/uart_fifo.sv sim/uart_fifo_tb.sv
    vvp build/uart_fifo_tb.vvp
elif command -v verilator >/dev/null 2>&1; then
    verilator --binary --timing -Wno-fatal --top-module uart_fifo_tb \
        --Mdir build/obj_uart_fifo -o uart_fifo_tb rtl/uart_fifo.sv sim/uart_fifo_tb.sv
    build/obj_uart_fifo/uart_fifo_tb
else
    echo "UART FIFOテスト用RTLシミュレータがありません。" >&2
    exit 2
fi
