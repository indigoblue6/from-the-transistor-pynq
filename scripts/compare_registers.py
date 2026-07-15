#!/usr/bin/env python3
"""エミュレータとRTLの最終レジスタ値を比較する。"""

from pathlib import Path
import re
import sys


def main() -> int:
    emulator_text = Path(sys.argv[1]).read_text(encoding="utf-8")
    emulator = {
        int(index): int(value, 16)
        for index, value in re.findall(r"r(\d+)=0x([0-9a-fA-F]{8})", emulator_text)
    }
    rtl_values = [int(line, 16) for line in Path(sys.argv[2]).read_text().splitlines()]
    if len(emulator) != 16 or len(rtl_values) != 16:
        print("レジスタダンプの形式が不正です", file=sys.stderr)
        return 1
    differences = [
        f"r{index}: emulator=0x{emulator[index]:08x}, rtl=0x{rtl_values[index]:08x}"
        for index in range(16)
        if emulator[index] != rtl_values[index]
    ]
    if differences:
        print("最終レジスタが一致しません:\n" + "\n".join(differences), file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
