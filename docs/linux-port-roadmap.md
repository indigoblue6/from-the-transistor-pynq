# Linux portロードマップ

## Phase A: privilege/trap/scheduler

本変更の段階。Kernel/User、precise trap、timer/external IRQ、syscall、2 User task preemption、fault
isolationを同一binaryのemulator/RTLとPYNQ-Z1で検証する。完了条件は`make test-all`と
`make hardware-kernel-test`、ILAでdouble faultなしを確認することである。

## Phase B: AXI4 masterとDDR

命令/data AXI4 master、PYNQ-Z1 DDR、boot ROM、UART/timer interrupt wiring、device tree草案を作る。
AXI protocol checker、DDR stress、実機DMA非干渉、boot ROMからDDR kernelへ遷移を完了条件とする。

## Phase C: MMU

page table walker、TLB、ASID、R/W/X/User permission、instruction/load/store page faultを実装する。
ランダムpage table differential test、TLB shootdown、copy-on-write fault、User仮想空間隔離で検証する。

## Phase D: toolchain/userspace

ELF32 ABI、relocation、linker、loader、compiler backendを整え、musl crt/libcとBusyBoxをbring-upする。
静的ELFのhello、fork/exec前提テスト、BusyBox ashをinitramfs上で実行する。

## Phase E: Linux arch/indigo

`arch/indigo` entry、irq、timekeeping、syscall、process context、uaccess、earlycon、device tree、initramfsを
実装する。kernel selftests、複数process、preemption、root shell到達を実機完了条件とする。

## Phase F: hybrid capability

32bit GPRは維持し、別系統64/128bit capability registerへbase、length、cursor、permissions、sealed、
tagを持たせる。通常integer storeはtagを消去する。DDR tagは別領域+tag cacheで管理し、MMUはprocess
隔離、capabilityはobject隔離を担当する。root操作はKernel専用、CAUSE 15で既存trapへ統合する。
compiler instrumentation、tag-loss、bounds/permission/seal fault、context switch保存を検証する。

## Phase G: pure capability

pure-capability ABI、capability-aware compiler/libc、ELF extension、pure-capability kernel/uaccessへ移行する。
整数pointer依存を排除したmusl/BusyBox、kernel selftests、攻撃回帰suiteを完了条件とする。
