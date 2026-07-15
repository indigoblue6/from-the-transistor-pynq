#!/usr/bin/env python3
"""FizzBuzz出力を独立に生成した期待値と照合する。"""

from pathlib import Path
import sys


def expected() -> str:
    lines: list[str] = []
    for value in range(1, 101):
        text = ""
        if value % 3 == 0:
            text += "Fizz"
        if value % 5 == 0:
            text += "Buzz"
        lines.append(text or str(value))
    return "\n".join(lines) + "\n"


def main() -> int:
    actual = Path(sys.argv[1]).read_text(encoding="ascii")
    if actual != expected():
        print("FizzBuzz出力が一致しません", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
