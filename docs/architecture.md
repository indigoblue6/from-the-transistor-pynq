# CPUアーキテクチャ

Indigo32はinstruction BRAMとdata BRAMを分けたmulti-cycle Harvard CPUである。FETCH、FETCH_WAIT、
DECODE、EXECUTE、MEMORY_REQUEST、MEMORY_WAIT、WFI、TRAP_WAIT、HALTED、FAULTのFSMを持つ。
割り込みはFETCH境界、同期例外はcommit前に確定する。STOREは保護・alignment・memory responseを
確認するまで書込みが発生せず、CALL/CSR/GPRも例外時に部分更新しない。

| RTL | 責務 |
|---|---|
| `cpu_indigo.sv` | FSM、precise commit、trap/ERET/WFI制御 |
| `csr_file.sv` | CSR、trap state、external pending |
| `timer.sv` | 64bit count/compare |
| `interrupt_controller.sv` | priorityとCAUSE生成 |
| `protection_unit.sv` | User base-limitとoverflow検査 |
| `decoder.sv` | field抽出、illegal/reserved分類 |
| `memory_map_indigo.sv` | BRAM、UART、SIM_EXIT |
| `pynq_z1_top.sv` | MMCM、PS bridge、UART、LED、ILA |

trap中の再trapはCSRの`trap_active`で検出しFAULTへ遷移する。外部にはmode、PC、instruction、trap、
CAUSE/EPC/TVAL、pending、timer/external IRQ、KERNEL_SP由来task ID、unrecoverableを出す。
CPUクロックはPYNQ-Z1の125 MHzから31.25 MHzを生成する。
