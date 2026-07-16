#!/bin/sh
set -eu

mkdir -p build
verilator --binary --timing -Wno-fatal --top-module uart_rx_tb \
    --Mdir build/obj_uart_rx -o uart_rx_tb rtl/uart_rx.sv sim/uart_rx_tb.sv
build/obj_uart_rx/uart_rx_tb
