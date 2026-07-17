#!/usr/bin/env python3
"""scheduler出力の実装依存な連続A/B数だけを正規化して比較する。"""

from __future__ import annotations

import re
import sys
from pathlib import Path


def normalize(data: bytes) -> bytes:
    data = re.sub(rb"\n[AB]+TRAP", b"\n<TASKS>TRAP", data)
    data = re.sub(rb"\n[AB]+SCHEDULER OK", b"\n<TASKS>SCHEDULER OK", data)
    return data


def main() -> int:
    if len(sys.argv) != 3:
        print("使い方: compare_scheduler.py EMULATOR RTL", file=sys.stderr)
        return 2
    emulator = normalize(Path(sys.argv[1]).read_bytes())
    rtl = normalize(Path(sys.argv[2]).read_bytes())
    if emulator != rtl:
        print("scheduler正規化出力が一致しません", file=sys.stderr)
        print(f"emulator={emulator!r}", file=sys.stderr)
        print(f"rtl={rtl!r}", file=sys.stderr)
        return 1
    print("scheduler差分テスト成功: timer量子内の連続A/B数のみ正規化")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
