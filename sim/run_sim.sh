#!/bin/sh
set -eu

program=${1:-hello}
mem="build/$program.mem"
case "$program" in
    hello) expected='Hello, PYNQ CPU!' ;;
    arithmetic) expected='OK' ;;
    branch) expected='0123456789' ;;
    call) expected='CALL OK' ;;
    rtl_test) expected='TEST OK' ;;
    *) expected='' ;;
esac
if [ "${SKIP_EXPECT:-0}" = 1 ]; then
    expected=""
fi
if [ -n "$expected" ]; then
    expected=$(printf '%s\nx' "$expected")
    expected=${expected%x}
fi
max_cycles=${MAX_CYCLES:-2000000}
output_arg=""
register_arg=""
external_arg=""
if [ -n "${OUTPUT_FILE:-}" ]; then
    output_arg="+OUTPUT_FILE=$OUTPUT_FILE"
fi
if [ "${EXTERNAL_IRQ:-0}" = 1 ]; then
    external_arg="+EXTERNAL_IRQ"
fi
if [ -n "${REGISTER_FILE:-}" ]; then
    register_arg="+REGISTER_FILE=$REGISTER_FILE"
fi

sources="rtl/alu.sv rtl/register_file.sv rtl/decoder.sv rtl/timer.sv rtl/interrupt_controller.sv rtl/csr_file.sv rtl/protection_unit.sv rtl/cpu_indigo.sv rtl/memory_map_indigo.sv sim/cpu_tb.sv"
mkdir -p build
if command -v iverilog >/dev/null 2>&1; then
    # Icarus Verilogを最優先する。
    iverilog -g2012 -s cpu_tb -o build/cpu_tb.vvp $sources
    vvp build/cpu_tb.vvp "+MEM=$mem" "+EXPECT=$expected" "+MAX_CYCLES=$max_cycles" $output_arg $register_arg $external_arg
elif command -v verilator >/dev/null 2>&1; then
    verilator --binary --timing -Wno-fatal --top-module cpu_tb \
        --Mdir build/obj_cpu -o cpu_tb $sources
    build/obj_cpu/cpu_tb "+MEM=$mem" "+EXPECT=$expected" "+MAX_CYCLES=$max_cycles" $output_arg $register_arg $external_arg
elif command -v xsim >/dev/null 2>&1; then
    echo "xsimは検出しましたが、バッチ実行環境はまだ未検証です。VivadoプロジェクトへRTLを追加してください。" >&2
    exit 2
else
    echo "iverilog、Verilator、xsimのいずれも見つかりません。" >&2
    exit 2
fi
