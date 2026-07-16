#!/usr/bin/env python3
"""PYNQ学習用CPUの2パスアセンブラ。"""

from __future__ import annotations

import argparse
import re
import struct
import sys
from dataclasses import dataclass
from pathlib import Path


OPCODES = {
    "nop": 0x00, "add": 0x01, "sub": 0x02, "and": 0x03,
    "or": 0x04, "xor": 0x05, "shl": 0x06, "shr": 0x07,
    "sar": 0x08, "movi": 0x09, "addi": 0x0A, "lui": 0x0B,
    "load": 0x0C, "store": 0x0D, "loadb": 0x0E, "storeb": 0x0F,
    "beq": 0x10, "bne": 0x11, "blt": 0x12, "bge": 0x13,
    "jmp": 0x14, "call": 0x15, "ret": 0x16, "halt": 0x17,
    "csrr": 0x18, "csrw": 0x19, "eret": 0x1A, "ecall": 0x1B,
    "wfi": 0x1C,
}
R_OPS = {"add", "sub", "and", "or", "xor", "shl", "shr", "sar"}
I_OPS = {"addi", "load", "store", "loadb", "storeb"}
B_OPS = {"beq", "bne", "blt", "bge"}
J_OPS = {"jmp", "call"}
ZERO_OPS = {"nop", "ret", "halt", "eret", "ecall", "wfi"}

CSRS = {
    "status": 0x00, "epc": 0x01, "cause": 0x02, "tvec": 0x03,
    "badaddr": 0x04, "timer_count_lo": 0x05, "timer_count_hi": 0x06,
    "timer_compare_lo": 0x07, "timer_compare_hi": 0x08,
    "interrupt_pending": 0x09, "interrupt_enable": 0x0A,
    "user_base": 0x0B, "user_limit": 0x0C, "kernel_sp": 0x0D,
    "scratch": 0x0E, "timer_control": 0x0F,
}


class AsmError(Exception):
    """入力ソースに起因するアセンブルエラー。"""


@dataclass
class SourceLine:
    number: int
    text: str
    body: str
    address: int = 0


def fail(line: SourceLine, message: str) -> AsmError:
    return AsmError(f"{line.number}行目: {message}\n    {line.text.rstrip()}")


def strip_comment(text: str) -> str:
    return re.split(r";|#|//", text, maxsplit=1)[0].strip()


def tokenize(body: str) -> list[str]:
    body = re.sub(r"([,\[\]+])", r" \1 ", body)
    return [token for token in body.split() if token not in {",", "[", "]", "+"}]


def parse_register(token: str, line: SourceLine) -> int:
    match = re.fullmatch(r"[rR](\d+)", token)
    if not match or not 0 <= int(match.group(1)) <= 15:
        raise fail(line, f"不正なレジスタ '{token}'（r0～r15を指定してください）")
    return int(match.group(1))


def parse_csr(token: str, symbols: dict[str, int], line: SourceLine) -> int:
    """CSR名または8 bit番号を解決する。"""
    key = token.lower()
    if key in CSRS:
        return CSRS[key]
    value = parse_value(token, symbols, line)
    if not 0 <= value <= 0xFF:
        raise fail(line, f"CSR番号 {value} は8 bitに収まりません")
    return value


def parse_value(token: str, symbols: dict[str, int], line: SourceLine) -> int:
    if token in symbols:
        return symbols[token]
    try:
        return int(token, 0)
    except ValueError:
        raise fail(line, f"未定義シンボルまたは不正な整数 '{token}'") from None


def checked(value: int, bits: int, line: SourceLine, what: str) -> int:
    lo, hi = -(1 << (bits - 1)), (1 << (bits - 1)) - 1
    if not lo <= value <= hi:
        raise fail(line, f"{what} {value} は符号付き{bits} bitに収まりません")
    return value & ((1 << bits) - 1)


def instruction_size(body: str, line: SourceLine) -> int:
    tokens = tokenize(body)
    op = tokens[0].lower()
    if op == "li":
        return 8
    if op in OPCODES:
        return 4
    raise fail(line, f"未定義命令 '{tokens[0]}'")


def parse_source(source: str) -> tuple[list[SourceLine], dict[str, int], int]:
    """第1パスでラベルと各行のアドレスを確定する。"""
    lines: list[SourceLine] = []
    symbols: dict[str, int] = {}
    pc = 0
    for number, raw in enumerate(source.splitlines(), 1):
        body = strip_comment(raw)
        line = SourceLine(number, raw, body, pc)
        while body:
            match = re.match(r"^([A-Za-z_.$][\w.$]*):\s*(.*)$", body)
            if not match:
                break
            name, body = match.groups()
            if name in symbols:
                raise fail(line, f"ラベル '{name}' が重複しています")
            symbols[name] = pc
            line.body = body
        line.address = pc
        line.body = body
        if not body:
            lines.append(line)
            continue
        tokens = tokenize(body)
        directive = tokens[0].lower()
        if directive == ".org":
            if len(tokens) != 2:
                raise fail(line, ".orgにはアドレスを1個指定します")
            new_pc = parse_value(tokens[1], symbols, line)
            if new_pc < pc:
                raise fail(line, ".orgで現在位置より前へ移動できません")
            pc = new_pc
        elif directive == ".align":
            if len(tokens) != 2:
                raise fail(line, ".alignにはバイト境界を1個指定します")
            alignment = parse_value(tokens[1], symbols, line)
            if alignment <= 0 or alignment & (alignment - 1):
                raise fail(line, ".alignは正の2のべき乗にしてください")
            pc = (pc + alignment - 1) & -alignment
        elif directive in {".word", ".byte"}:
            if len(tokens) < 2:
                raise fail(line, f"{directive}には値を1個以上指定します")
            pc += (4 if directive == ".word" else 1) * (len(tokens) - 1)
        elif directive.startswith("."):
            raise fail(line, f"未定義ディレクティブ '{tokens[0]}'")
        else:
            pc += instruction_size(body, line)
        lines.append(line)
    return lines, symbols, pc


def encode_instruction(line: SourceLine, symbols: dict[str, int]) -> list[int]:
    tokens = tokenize(line.body)
    op = tokens[0].lower()

    def count(n: int, syntax: str) -> None:
        if len(tokens) != n:
            raise fail(line, f"構文: {syntax}")

    if op == "li":
        count(3, "li rd, value")
        rd, value = parse_register(tokens[1], line), parse_value(tokens[2], symbols, line)
        if not -(1 << 31) <= value <= 0xFFFFFFFF:
            raise fail(line, "LIの値は32 bitに収めてください")
        value &= 0xFFFFFFFF
        low = value & 0xFFFF
        signed_low = low if low < 0x8000 else low - 0x10000
        high = ((value - signed_low) >> 16) & 0xFFFF
        return [
            (OPCODES["lui"] << 26) | (rd << 22) | high,
            (OPCODES["addi"] << 26) | (rd << 22) | (rd << 18) | (signed_low & 0x3FFFF),
        ]
    if op not in OPCODES:
        raise fail(line, f"未定義命令 '{tokens[0]}'")
    opcode = OPCODES[op] << 26
    if op in ZERO_OPS:
        count(1, op)
        return [opcode]
    if op in R_OPS:
        count(4, f"{op} rd, rs1, rs2")
        rd, rs1, rs2 = (parse_register(t, line) for t in tokens[1:])
        return [opcode | (rd << 22) | (rs1 << 18) | (rs2 << 14)]
    if op == "csrr":
        count(3, "csrr rd, csr")
        rd = parse_register(tokens[1], line)
        csr = parse_csr(tokens[2], symbols, line)
        return [opcode | (rd << 22) | csr]
    if op == "csrw":
        count(3, "csrw csr, rs")
        csr = parse_csr(tokens[1], symbols, line)
        rs = parse_register(tokens[2], line)
        return [opcode | (rs << 22) | csr]
    if op == "movi":
        count(3, "movi rd, imm22")
        rd = parse_register(tokens[1], line)
        imm = checked(parse_value(tokens[2], symbols, line), 22, line, "即値")
        return [opcode | (rd << 22) | imm]
    if op == "lui":
        count(3, "lui rd, imm16")
        rd, imm = parse_register(tokens[1], line), parse_value(tokens[2], symbols, line)
        if not -0x8000 <= imm <= 0xFFFF:
            raise fail(line, "LUIの即値は16 bitに収めてください")
        return [opcode | (rd << 22) | (imm & 0xFFFF)]
    if op in I_OPS:
        count(4, f"{op} reg, [base + offset]")
        reg, base = parse_register(tokens[1], line), parse_register(tokens[2], line)
        imm = checked(parse_value(tokens[3], symbols, line), 18, line, "オフセット")
        return [opcode | (reg << 22) | (base << 18) | imm]
    if op in B_OPS:
        count(4, f"{op} rs1, rs2, target")
        rs1, rs2 = parse_register(tokens[1], line), parse_register(tokens[2], line)
        delta = parse_value(tokens[3], symbols, line) - (line.address + 4)
        if delta % 4:
            raise fail(line, "分岐先が4 byte境界ではありません")
        return [opcode | (rs1 << 22) | (rs2 << 18) | checked(delta // 4, 18, line, "分岐オフセット")]
    if op in J_OPS:
        count(2, f"{op} target")
        delta = parse_value(tokens[1], symbols, line) - (line.address + 4)
        if delta % 4:
            raise fail(line, "ジャンプ先が4 byte境界ではありません")
        return [opcode | checked(delta // 4, 26, line, "ジャンプオフセット")]
    raise fail(line, f"'{op}'のエンコーダがありません")


def assemble(source: str) -> bytes:
    """第2パスで命令とデータをリトルエンディアンのバイト列へ変換する。"""
    lines, symbols, final_size = parse_source(source)
    output, pc = bytearray(), 0
    for line in lines:
        if not line.body:
            continue
        tokens = tokenize(line.body)
        op = tokens[0].lower()
        if op == ".org":
            target = parse_value(tokens[1], symbols, line)
            output.extend(b"\0" * (target - pc)); pc = target
        elif op == ".align":
            alignment = parse_value(tokens[1], symbols, line)
            target = (pc + alignment - 1) & -alignment
            output.extend(b"\0" * (target - pc)); pc = target
        elif op == ".word":
            for token in tokens[1:]:
                value = parse_value(token, symbols, line)
                if not -(1 << 31) <= value <= 0xFFFFFFFF:
                    raise fail(line, ".wordの値は32 bitに収めてください")
                output.extend(struct.pack("<I", value & 0xFFFFFFFF)); pc += 4
        elif op == ".byte":
            for token in tokens[1:]:
                value = parse_value(token, symbols, line)
                if not -128 <= value <= 255:
                    raise fail(line, ".byteの値は8 bitに収めてください")
                output.append(value & 0xFF); pc += 1
        else:
            if pc % 4:
                raise fail(line, "命令が4 byte境界ではありません")
            for word in encode_instruction(line, symbols):
                output.extend(struct.pack("<I", word)); pc += 4
    assert len(output) == final_size
    return bytes(output)


def write_mem(data: bytes, path: Path) -> None:
    """$readmemh用に32 bitワードを1行ずつ書き出す。"""
    padded = data + b"\0" * (-len(data) % 4)
    words = (struct.unpack_from("<I", padded, offset)[0] for offset in range(0, len(padded), 4))
    path.write_text("".join(f"{word:08x}\n" for word in words), encoding="ascii")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("input", type=Path)
    parser.add_argument("-o", "--output", type=Path, required=True)
    parser.add_argument("--mem", type=Path, help="BRAM初期化用MEMも出力する")
    args = parser.parse_args(argv)
    try:
        data = assemble(args.input.read_text(encoding="utf-8"))
        args.output.parent.mkdir(parents=True, exist_ok=True); args.output.write_bytes(data)
        if args.mem:
            args.mem.parent.mkdir(parents=True, exist_ok=True); write_mem(data, args.mem)
    except (AsmError, OSError) as error:
        print(f"assembler: エラー: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
