#!/usr/bin/env bash
set -euo pipefail

mkdir -p build
python3 assembler/assembler.py kernel/scheduler_demo.s \
    -o build/scheduler.bin --mem build/scheduler.mem
cargo run --quiet --manifest-path emulator/Cargo.toml -- \
    build/scheduler.bin --max-steps 2000000 >build/scheduler-emulator.out
SKIP_EXPECT=1 MAX_CYCLES=2000000 OUTPUT_FILE=build/scheduler-rtl.out \
    sh sim/run_sim.sh scheduler
for output in build/scheduler-emulator.out build/scheduler-rtl.out; do
    for marker in "KERNEL BOOT" "USER MODE OK" \
        "TRAP cause=STORE_ACCESS_FAULT EPC=0x00002044 TVAL=0x00009000" \
        "BAD TASK KILLED" "SCHEDULER OK"; do
        grep -F "$marker" "$output" >/dev/null || {
            echo "$output に不足: $marker" >&2
            exit 1
        }
    done
    python3 - "$output" <<'PY'
import re, sys
text = open(sys.argv[1], encoding="ascii").read()
before, after = text.split("BAD TASK KILLED\n", 1)
assert "A" in before and "B" in before, "fault前にA/Bの両方が動いていません"
assert "A" in after and "B" in after, "fault後にA/Bの両方が継続していません"
PY
done
python3 scripts/compare_scheduler.py \
    build/scheduler-emulator.out build/scheduler-rtl.out
echo "Kernel/User scheduler統合テスト成功"
