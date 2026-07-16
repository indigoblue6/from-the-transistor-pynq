# Indigo32例外・割り込みモデル

## 基本方針

例外は命令境界で確定し、原因命令がレジスタまたはメモリを部分更新しないprecise trapとする。
割り込みは命令と命令の間でのみ受理し、メモリ要求の途中では受理しない。例外のネストは
IndigoOS 0.1では扱わない。`TVEC`が0のまま同期faultが起きた場合は、第1フェーズとの互換性のため
例外ベクタへ移らずterminal fault状態へ入る。

## 追加命令

既存opcode `0x00`から`0x17`までは変更しない。

| opcode | 命令 | エンコード | 動作 |
|---:|---|---|---|
| `0x18` | `CSRR rd, csr` | rd=`[25:22]`, csr=`[7:0]`, `[21:8]=0` | CSRを読む |
| `0x19` | `CSRW csr, rs` | rs=`[25:22]`, csr=`[7:0]`, `[21:8]=0` | CSRへ書く |
| `0x1a` | `ERET` | `[25:0]=0` | `EPC`へ復帰し状態を復元する |
| `0x1b` | `ECALL` | `[25:0]=0` | environment call例外を起こす |
| `0x1c` | `WFI` | `[25:0]=0` | 有効な割り込みまで実行を待機する |

`CSRW`と`ERET`はKernel mode専用である。User modeで実行すると特権命令例外になる。
`CSRR`は`TIMER_COUNT_LO/HI`だけUser modeから読め、その他はKernel mode専用とする。

## CSR一覧

| 番号 | 名称 | リセット値 | 権限 | 意味 |
|---:|---|---:|---|---|
| `0x00` | `STATUS` | `0x00000004` | K R/W | 実行状態 |
| `0x01` | `EPC` | 0 | K R/W | 例外復帰PC |
| `0x02` | `CAUSE` | 0 | K R | 例外・割り込み原因 |
| `0x03` | `TVEC` | 0 | K R/W | 例外入口。4 byte境界必須 |
| `0x04` | `BADADDR` | 0 | K R | 原因アドレスまたは命令語 |
| `0x05` | `TIMER_COUNT_LO` | 0 | U/K R | 64 bit単調タイマー下位 |
| `0x06` | `TIMER_COUNT_HI` | 0 | U/K R | 64 bit単調タイマー上位 |
| `0x07` | `TIMER_COMPARE_LO` | `0xffffffff` | K R/W | 比較値下位 |
| `0x08` | `TIMER_COMPARE_HI` | `0xffffffff` | K R/W | 比較値上位 |
| `0x09` | `INTERRUPT_PENDING` | 0 | K R/W1C | pending bit |
| `0x0a` | `INTERRUPT_ENABLE` | 0 | K R/W | 個別許可bit |
| `0x0b` | `USER_BASE` | 0 | K R/W | User mode許可領域先頭 |
| `0x0c` | `USER_LIMIT` | 0 | K R/W | User mode許可領域末尾の次 |
| `0x0d` | `KERNEL_SP` | `0x00010000` | K R/W | trap入口用カーネルSP |
| `0x0e` | `SCRATCH` | 0 | K R/W | CPU依存入口の一時値 |
| `0x0f` | `TIMER_CONTROL` | 0 | K R/W | bit 0でタイマー割り込み有効 |

`STATUS`のbit 0はIE、bit 1はPIE、bit 2はPRIV、bit 3はPPRIVである。PRIVが1なら
Kernel mode、0ならUser modeである。未定義bitは0を書き、0として読む。

`INTERRUPT_PENDING`と`INTERRUPT_ENABLE`はbit 0がtimer、bit 1がUART RX、bit 2がsoftware
interruptである。pendingは割り込み禁止中も保持する。W1C書込みでは1を書いたbitだけ解除する。
レベル条件が継続しているtimerとUART RXは次のサイクルに再設定される。

## 原因番号

| 値 | 原因 |
|---:|---|
| 0 | illegal instruction |
| 1 | instruction address misaligned |
| 2 | instruction access fault |
| 3 | load address misaligned |
| 4 | load access fault |
| 5 | store address misaligned |
| 6 | store access fault |
| 7 | environment call |
| 8 | timer interrupt |
| 9 | UART RX interrupt |
| 10 | software interrupt |
| 11 | privileged instruction |

非同期原因は8以上とし、IndigoOS 0.1では固定優先順位をtimer、UART RX、softwareの順とする。

## 例外入口と復帰

例外入口では、`PIE←IE`、`PPRIV←PRIV`、`IE←0`、`PRIV←Kernel`とし、`CAUSE`、
`BADADDR`、`EPC`を確定して`PC←TVEC`とする。不正命令、fetch、LOAD、STOREでは原因命令の
アドレスをEPCへ保存する。ECALLでは次命令、割り込みではまだ実行していない次命令を保存する。

`ERET`は`PC←EPC`、`IE←PIE`、`PRIV←PPRIV`として復帰する。復帰後はPIEを0、PPRIVを1へ
戻す。OSが同期faultを再実行しない場合は、ハンドラ内でEPCを更新する。

## タイマー

64 bit countはCPUクロックごとに増える。`TIMER_CONTROL.bit0=1`かつ
`COUNT >= COMPARE`でtimer pendingを設定する。COMPAREを将来値へ更新するとpending条件が
解除される。周期動作はハンドラが次のCOMPAREを設定して作る。実機の100 Hz tick値はトップの
CPUクロック周波数からMake/Vivado設定で導出し、31.25 MHz時は312,500 cycleである。

