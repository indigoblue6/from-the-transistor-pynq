#!/usr/bin/env python3
"""Python・Rust・SystemVerilog間のISA定数を照合する。"""

from __future__ import annotations

import importlib.util
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def load_assembler():
    spec = importlib.util.spec_from_file_location("indigo_assembler", ROOT / "assembler/assembler.py")
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def rust_constants(text: str, module: str) -> dict[str, int]:
    body = re.search(rf"pub mod {module} \{{(.*?)\n\}}", text, re.S)
    assert body, f"Rust module {module} が見つかりません"
    return {
        name: int(value, 16)
        for name, value in re.findall(r"pub const ([A-Z0-9_]+): u8 = 0x([0-9a-fA-F]+);", body.group(1))
    }


def sv_constants(text: str, prefix: str, width: int) -> dict[str, int]:
    return {
        name.removeprefix(prefix): int(value, 16)
        for name, value in re.findall(rf"\b({prefix}[A-Z0-9_]+)\s*=\s*{width}'h([0-9a-fA-F]+)", text)
    }


def require_equal(label: str, expected: dict[str, int], actual: dict[str, int]) -> None:
    missing = expected.keys() - actual.keys()
    mismatched = {key: (expected[key], actual[key]) for key in expected.keys() & actual.keys() if expected[key] != actual[key]}
    assert not missing and not mismatched, f"{label}: missing={sorted(missing)}, mismatch={mismatched}"


def require_subset(label: str, expected: dict[str, int], actual: dict[str, int]) -> None:
    unknown = actual.keys() - expected.keys()
    mismatched = {key: (expected[key], actual[key]) for key in actual.keys() & expected.keys() if expected[key] != actual[key]}
    assert not unknown and not mismatched, f"{label}: unknown={sorted(unknown)}, mismatch={mismatched}"


def main() -> None:
    assembler = load_assembler()
    rust = (ROOT / "emulator/src/machine.rs").read_text()
    decoder = (ROOT / "rtl/decoder.sv").read_text()
    csr_file = (ROOT / "rtl/csr_file.sv").read_text()
    cpu = (ROOT / "rtl/cpu_indigo.sv").read_text()

    py_opcodes = {name.upper(): value for name, value in assembler.OPCODES.items()}
    require_equal("assembler/Rust opcode", py_opcodes, rust_constants(rust, "opcode"))
    require_subset("assembler/decoder opcode", py_opcodes, sv_constants(decoder, "OP_", 6))

    py_csrs = {name.upper(): value for name, value in assembler.CSRS.items()}
    require_equal("assembler/Rust CSR", py_csrs, rust_constants(rust, "csr"))
    require_equal("assembler/csr_file CSR", py_csrs, sv_constants(csr_file, "CSR_", 8))

    rust_causes = {
        name: int(value)
        for name, value in re.findall(r"pub const ([A-Z0-9_]+): u32 = (\d+);", re.search(r"pub mod cause \{(.*?)\n\}", rust, re.S).group(1))
    }
    rtl_causes = {
        name.removeprefix("CAUSE_"): int(value)
        for name, value in re.findall(r"\b(CAUSE_[A-Z0-9_]+)\s*=\s*32'd(\d+)", cpu)
    }
    cause_alias = {
        "ILLEGAL_INSTRUCTION": "ILLEGAL", "INSTRUCTION_MISALIGNED": "FETCH_MISALIGNED",
        "INSTRUCTION_ACCESS": "FETCH_ACCESS", "LOAD_MISALIGNED": "LOAD_MISALIGNED",
        "LOAD_ACCESS": "LOAD_ACCESS", "STORE_MISALIGNED": "STORE_MISALIGNED",
        "STORE_ACCESS": "STORE_ACCESS", "ECALL": "ECALL", "PRIVILEGED_INSTRUCTION": "PRIVILEGED",
        "USER_ECALL": "USER_ECALL", "RESERVED_ENCODING": "RESERVED_ENCODING",
    }
    expected_causes = {rtl_name: rust_causes[rust_name] for rust_name, rtl_name in cause_alias.items()}
    require_equal("Rust/RTL cause", expected_causes, rtl_causes)
    print("ISA定数整合性テスト成功: opcode/CSR/CAUSE")


if __name__ == "__main__":
    main()
