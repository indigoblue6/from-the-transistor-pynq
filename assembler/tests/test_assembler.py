import struct
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).parents[1]))
from assembler import AsmError, OPCODES, assemble, write_mem  # noqa: E402


def words(source: str) -> tuple[int, ...]:
    data = assemble(source)
    return struct.unpack(f"<{len(data) // 4}I", data)


def test_r形式と即値形式のエンコード():
    assert words("add r1, r2, r3") == ((OPCODES["add"] << 26) | (1 << 22) | (2 << 18) | (3 << 14),)
    assert words("movi r4, -1") == ((OPCODES["movi"] << 26) | (4 << 22) | 0x3FFFFF,)
    assert words("store r5, [r15 + -4]") == ((OPCODES["store"] << 26) | (5 << 22) | (15 << 18) | 0x3FFFC,)


def test_ラベルは次命令pcからの相対値になる():
    encoded = words("start: nop\nbne r1, r0, start\ncall target\nhalt\ntarget: ret\n")
    assert encoded[1] & 0x3FFFF == 0x3FFFE
    assert encoded[2] & 0x3FFFFFF == 1


@pytest.mark.parametrize("value", [0, 0x7FFF, 0x8000, 0x1234FFFF, 0x80000000, 0xFFFFFFFF, -1])
def test_liで任意の32bit値を構築できる(value):
    lui, addi = words(f"li r7, {value}")
    high, low18 = lui & 0xFFFF, addi & 0x3FFFF
    low = low18 - (1 << 18) if low18 & (1 << 17) else low18
    assert ((high << 16) + low) & 0xFFFFFFFF == value & 0xFFFFFFFF


def test_データディレクティブとmem出力(tmp_path):
    data = assemble(".byte 1, -1\n.align 4\n.word 0x12345678\n.org 12\n.byte 2")
    assert data == b"\x01\xff\0\0\x78\x56\x34\x12\0\0\0\0\x02"
    path = tmp_path / "image.mem"
    write_mem(data, path)
    assert path.read_text().splitlines()[:2] == ["0000ff01", "12345678"]


def test_エラーに行番号を含む():
    with pytest.raises(AsmError, match=r"2行目: 不正なレジスタ"):
        assemble("nop\nadd r16, r0, r0")
    with pytest.raises(AsmError, match=r"1行目: 即値"):
        assemble("movi r1, 99999999")


def test_csr_and_trap_instructions():
    data = assemble(
        "csrr r3, status\ncsrw tvec, r4\neret\necall\nwfi\n"
    )
    words = struct.unpack("<5I", data)
    assert words == (
        (0x18 << 26) | (3 << 22),
        (0x19 << 26) | (4 << 22) | 3,
        0x1A << 26,
        0x1B << 26,
        0x1C << 26,
    )


def test_invalid_csr_number_reports_line():
    with pytest.raises(AsmError) as error:
        assemble("csrr r1, 0x100\n")
    assert "1行目" in str(error.value)
    assert "CSR番号" in str(error.value)
