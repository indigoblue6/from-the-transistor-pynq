#!/bin/sh
set -eu

mkdir -p build
python3 assembler/assembler.py sim/programs/exception_test.s \
    -o build/exception_test.bin --mem build/exception_test.mem
SKIP_EXPECT=1 MAX_CYCLES=200000 sh sim/run_sim.sh exception_test
