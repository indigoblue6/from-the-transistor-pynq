# Indigo32特権アーキテクチャ

## モードとリセット

CPUはKernel modeとUser modeを持つ。`STATUS.PRIV=1`がKernel、0がUserであり、リセット値は
`0x00000004`なのでKernelで開始する。User modeの`HALT`、`WFI`、CSR書込み、`ERET`、timer/IRQ
制御、TVEC変更、Kernel専用MMIOおよび保護範囲外アクセスはCAUSE 11のprivilege/access trapとなる。
CPU全体は停止しない。将来のMMU制御とcapability root操作もKernel専用とする。

## STATUS

| bit | 名称 | 意味 |
|---:|---|---|
| 0 | IE | global interrupt enable |
| 1 | PIE | trap前のIE |
| 2 | PRIV | 現在モード。1=Kernel、0=User |
| 3 | PPRIV | trap前のモード |

trap入口は`PIE←IE, PPRIV←PRIV, IE←0, PRIV←1`を原子的に行う。ERETはEPCへ復帰し、
`IE←PIE, PRIV←PPRIV, PIE←0, PPRIV←1`とする。ERETはKernel専用である。

## 暫定保護

`protection_unit.sv`が`[USER_BASE, USER_LIMIT)`の全byteを検査する。32bit加算のcarry、
`0x80000000`以上、範囲外fetch/load/storeを拒否する。scheduler demoはUser code/dataを
`0x2000–0x7fff`、TCB/kernel stackを`0x9000`以上へ置く。これは単一物理範囲のPMP風保護で、
タスク相互のアドレス空間分離ではない。将来は`allow = PMP && MMU && capability`へ拡張する。
