# Indigo32命令セット

Indigo32はlittle endian、32bit固定長、4 byte命令整列、16本の32bit GPR（r0=0）を持つ。
既存opcode `0x00–0x17`とそのencodingは変更しない。R/I/M/B/J形式、PC相対基準、LOAD/STOREの
意味論も従来どおりである。

## opcode

| 値 | 命令 | encoding/意味 |
|---:|---|---|
| `00` | NOP | 全予約bit 0 |
| `01–08` | ADD/SUB/AND/OR/XOR/SHL/SHR/SAR | R形式、`[13:0]=0` |
| `09` | MOVI | rd + signed imm22 |
| `0a` | ADDI | rd, rs + signed imm18 |
| `0b` | LUI | rd + unsigned imm16、`[21:16]=0` |
| `0c–0f` | LOAD/STORE/LOADB/STOREB | reg, base, signed offset18 |
| `10–13` | BEQ/BNE/BLT/BGE | 2 register + word offset18 |
| `14–15` | JMP/CALL | word offset26、CALLはr14更新 |
| `16` | RET | PC=r14 |
| `17` | HALT | Kernel専用 |
| `18` | CSRR | `rd=[25:22], csr=[7:0], [21:8]=0` |
| `19` | CSRW | `rs=[25:22], csr=[7:0], [21:8]=0` |
| `1a` | ERET | Kernel専用、予約bit 0 |
| `1b` | ECALL | mode別syscall trap、予約bit 0 |
| `1c` | WFI | Kernel専用、予約bit 0 |
| `1d` | CSRSET | `CSR←CSR OR rs`、Kernel専用 |
| `1e` | CSRCLR | `CSR←CSR AND NOT rs`、Kernel専用 |

CSRSET/CLRはread-only CSRとW1C `INTERRUPT_PENDING`には使用できず、illegal trapとなる。CSRRは
UserからTIMER_COUNT_LO/HIだけ許可する。未知opcodeはillegal、既知opcodeの非0予約bitはreserved
encoding trapである。assemblerは未実装CSR番号を拒否する。

## CSR

| # | 名称 | reset | 権限/属性 |
|---:|---|---:|---|
| 00 | STATUS | `00000004` | K R/W |
| 01 | EPC | 0 | K R/W |
| 02 | CAUSE | 0 | K R |
| 03 | TVEC | 0 | K R/W、4 byte整列 |
| 04 | TVAL/BADADDR | 0 | K R |
| 05/06 | TIMER_COUNT_LO/HI | 0 | U/K R |
| 07/08 | TIMER_COMPARE_LO/HI | all-one | K R/W |
| 09 | INTERRUPT_PENDING | 0 | K R/W1C |
| 0a | INTERRUPT_ENABLE | 0 | K R/W |
| 0b/0c | USER_BASE/USER_LIMIT | 0 | K R/W |
| 0d | KERNEL_SP | `00010000` | K R/W |
| 0e | SCRATCH | 0 | K R/W |
| 0f | TIMER_CONTROL | 0 | K R/W、bit0 enable |

`0x40–0x7f`を将来のcapability CSR空間として予約し、今回のassembler/CPUはアクセスを拒否する。
CAUSEは[traps.md](traps.md)、STATUSは[privilege.md](privilege.md)を参照する。
