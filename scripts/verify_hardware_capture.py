#!/usr/bin/env python3
"""Vivado ILAのCSVから実機UART履歴とCPU終了状態を検証する。"""

import argparse
import csv
import re
import sys
from pathlib import Path


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--kernel", action="store_true",
                        help="scheduler kernelの終了状態とUART末尾を検証する")
    args = parser.parse_args()
    path = Path("build/hardware/hardware_capture.csv")
    with path.open(newline="", encoding="utf-8") as stream:
        rows = csv.reader(stream)
        header = next(rows)
        next(rows)  # radix指定行
        sample = next(rows)

    value = 0
    for name, field in zip(header, sample):
        match = re.search(r"uart_history(?:_\d+)?\[(\d+):(\d+)\]", name)
        if match:
            _high, low = map(int, match.groups())
            value |= int(field, 16) << low

    actual = value.to_bytes(17, "big")
    expected = b"SCHEDULER OK\n" if args.kernel else b"Hello, PYNQ CPU!\n"
    halted = sample[header.index("halted_internal")]
    faulted = sample[header.index("faulted_internal")]
    uart_ok = actual.endswith(expected) if args.kernel else actual == expected
    unrecoverable = "0"
    if args.kernel and "unrecoverable_fault_debug" in header:
        unrecoverable = sample[header.index("unrecoverable_fault_debug")]
    if not uart_ok or halted != "1" or faulted != "0" or unrecoverable != "0":
        print(
            f"実機検証失敗: UART={actual!r}, halt={halted}, fault={faulted}, "
            f"unrecoverable={unrecoverable}", file=sys.stderr,
        )
        return 1
    print(f"実機検証成功: UART末尾={expected!r}, halt=1, fault=0")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
