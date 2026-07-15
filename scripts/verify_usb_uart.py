#!/usr/bin/env python3
"""外付け3.3 V USB-UARTでPYNQ-Z1のPL UART出力を検証する。"""

import argparse
import os
import select
import sys
import termios
import time
import tty


EXPECTED = b"Hello, PYNQ CPU!\n"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("port", help="外付けUSB-UARTのデバイス（例: /dev/ttyUSB2）")
    parser.add_argument("--timeout", type=float, default=30.0)
    args = parser.parse_args()

    fd = os.open(args.port, os.O_RDWR | os.O_NOCTTY | os.O_NONBLOCK)
    try:
        tty.setraw(fd)
        settings = termios.tcgetattr(fd)
        settings[4] = termios.B115200
        settings[5] = termios.B115200
        settings[2] &= ~(termios.PARENB | termios.CSTOPB | termios.CSIZE)
        settings[2] |= termios.CS8 | termios.CLOCAL | termios.CREAD
        termios.tcsetattr(fd, termios.TCSANOW, settings)
        termios.tcflush(fd, termios.TCIFLUSH)
        received = bytearray()
        deadline = time.monotonic() + args.timeout
        while time.monotonic() < deadline and EXPECTED not in received:
            readable, _, _ = select.select([fd], [], [], 0.2)
            if readable:
                received.extend(os.read(fd, 4096))
        if EXPECTED not in received:
            print(f"USB-UART検証失敗: 受信={bytes(received)!r}", file=sys.stderr)
            return 1
        print(f"USB-UART実機検証成功: 受信={EXPECTED!r}")
        return 0
    finally:
        os.close(fd)


if __name__ == "__main__":
    raise SystemExit(main())
