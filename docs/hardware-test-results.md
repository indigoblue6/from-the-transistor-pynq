# 実機テスト記録

## 2026-07-16 PYNQ-Z1

- ツール: AMD Vivado 2025.2
- FPGA: XC7Z020-1CLG400
- 接続: Digilent USB-JTAG
- CPUクロック: 31.25 MHz
- プログラム: `examples/hello.s`
- bitstream書込み: 成功
- ILAトリガ: `halted_internal=1`で成功
- UART履歴: `Hello, PYNQ CPU!\n`と完全一致
- CPU状態: halt=1、fault=0
- 配線後タイミング: WNS 2.850 ns、TNS 0、WHS 0.035 ns、THS 0
- Block RAM: RAMB36E1 20個、RAMB18E1 1個（ILA分を含む）

外付け3.3 V USB-UART変換器はこの実行時には接続されていない。接続済みの`ttyUSB`は
PYNQ-Z1内蔵Digilent Adeptで、PLのPMODA JA1には接続されていないため、物理UART線の受信試験は
未実施である。外付け変換器を接続後、`make hardware-uart-test PORT=/dev/ttyUSBx`で追試する。

## 2026-07-16 PynqC実機テスト

`make hardware-c-test`で`examples/c/hardware.pc`をPynqCコンパイルし、同じ既存アセンブラで
機械語化したROMをPYNQ-Z1へ書き込んだ。Vivado 2025.2の合成、配置、配線、bitstream生成は
エラー0、Critical Warning 0で完了した。最終タイミングはWNS `+2.484 ns`、TNS `0 ns`、
WHS `+0.035 ns`、THS `0 ns`である。

JTAG/ILAで`UART=b"Hello, PYNQ CPU!\n"`、`halt=1`、`fault=0`を確認した。これにより
PynqC → 独自ISAアセンブリ → 機械語 → 実機CPU → UART FIFO受理までを実機で検証した。
外付けUSB-UART端子での物理波形受信は、ケーブル未所持のため引き続き未実施である。

## 2026-07-17 Kernel/User scheduler milestone

- Verilator: scheduler同一binaryが`KERNEL BOOT`、User A/B、timer切替、store access fault、bad task終了、
  fault後のA/B継続、`SCHEDULER OK`を出力した。
- Rust参照機: 同じbinaryと同じ構造化出力を確認した。timer単位差によるA/B連続数だけを正規化して一致した。
- Vivado 2025.2: 16-probe ILAを含むsynthesis/place/route/bitstream生成に成功した。最終routeは
  WNS `+0.508 ns`、TNS `0`、WHS `+0.026 ns`、THS `0`、unrouted net 0、Bitgen DRC error 0だった。
  BRAM使用量は29.5 tile（21.07%）である。
- PYNQ-Z1へbitstreamを書き込み、`hardware_done=1`でILA triggerした。ILAのUART履歴末尾は
  `SCHEDULER OK\n`、`halt=1`、`fault=0`、`unrecoverable=0`だった。最終CAUSEはtimer interrupt
  `0x80000008`、EPCは`0x0000202c`、KERNEL_SP由来task IDは1を観測した。
- 初回の32-byte物理UART FIFOではscheduler出力後半をdropしたため、256 byteへ拡張して再合成・
  再書込みした。上記の最終結果は拡張後bitstreamによるものである。

これによりJTAG/ILA経路でKernel boot、User実行、syscall UART出力、timer preemption、bad task隔離後の
scheduler継続、double faultなしを実機確認した。外付けUSB-UART変換器がPL UART端子に接続されて
いないため、物理UART線上の全出力受信だけは未実施である。

## 2026-07-17 Digilent Adept PS UART検証

Digilent Adept内蔵PS UART (`/dev/ttyUSB1`) を115200 baud、8N1、flow controlなしで使用し、
`PS BRIDGE READY`に続いてschedulerの全ログを実受信した。最終行は`SCHEDULER OK`だった。
初期bridgeのstack `0x0003fff0`は低位OCM外で、`main`先頭のpushがData Abortしていた。
stackを`0x0002fff0`へ変更し、XSCTでprocessor reset後にELFを開始することで解消した。
以後は`make hardware-kernel-console`を通常の対話デバッグ手順とする。
