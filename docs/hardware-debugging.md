# PYNQ-Z1実機デバッグ手順

## 前提

PYNQ-Z1の電源を入れ、USB-JTAGを開発PCへ接続する。ブートジャンパはJTAGモードを推奨する。
Vivado 2025.2以降をPATHへ追加し、次を確認する。

```bash
command -v vivado
lsusb | grep 0403:6010
```

FT2232が見えない場合はUSBケーブル、電源、udev権限を確認する。別のVivado Hardware Managerや
`hw_server`がJTAGを占有している場合は終了する。

## 一括テスト

```bash
make hardware-test
# 外付けUSB-UARTの受信を同時検証する場合
make hardware-uart-test PORT=/dev/ttyUSB2
```

処理内容は次のとおりである。

1. `hello.s`を`build/hello.mem`へアセンブルする。
2. XC7Z020向けに合成、ILA挿入、配置配線、bitstream生成を行う。
3. PYNQ-Z1へJTAG書込みする。
4. `halted_internal=1`をILAトリガとして1024サンプル取得する。
5. UART履歴が`Hello, PYNQ CPU!\n`、haltが1、faultが0であることを厳密比較する。

成功時は`実機検証成功`と表示する。生成物は`build/hardware/`にあり、タイミングは
`timing_summary.rpt`、使用量は`utilization.rpt`、ILA波形は`hardware_capture.csv`である。

## 段階的な実行

合成と実機書込みを分ける場合は次を使う。

```bash
make hardware-build
vivado -mode batch -source scripts/program_hardware.tcl -nojournal -nolog
python3 scripts/verify_hardware_capture.py
```

Vivado GUIで見る場合は`build/hardware/pynq_cpu.xpr`を開き、Hardware Managerでターゲットへ
接続する。Program Deviceで`pynq_cpu.bit`と`pynq_cpu.ltx`を指定する。ILAのprobeは次の3本である。

| probe | 意味 | 正常値 |
|---|---|---|
| `halted_internal` | HALT到達 | `1` |
| `faulted_internal` | ISAまたはメモリfault | `0` |
| `uart_history[135:0]` | 直近17 byte | `Hello, PYNQ CPU!\n` |

UART履歴は古いbyteが上位、新しいbyteが下位に入る。正常値の16進表現は
`48656c6c6f2c2050594e5120435055210a`である。

## LEDと物理UART

| 信号 | 意味 |
|---|---|
| LED0 | 正常HALT |
| LED1 | fault |
| LED2 | UART送信器ready |
| LED3 | 31.25 MHzクロックのheartbeat |

UART TXはPMODA JA1へ115200 baud、8 data bit、パリティなし、1 stop bitで出る。3.3 V対応の
USB-UART変換器を使い、PYNQ-Z1とGNDを共有する。変換器のTXをJA1へ接続してはならない。

```bash
picocom -b 115200 /dev/ttyUSBX
```

基板内蔵USB-UARTはPSのMIO側であり、本プロジェクトのPL-only UARTとは別物である。

## fault時の切り分け

LED1または`faulted_internal`が1なら、まず同じMEMを参照エミュレータでトレースする。

```bash
cargo run --manifest-path emulator/Cargo.toml -- build/hello.bin --trace --dump-registers
```

- 最初の命令でfault: MEM初期化ファイル、予約bit、PCアラインメントを確認する。
- LOAD/STOREでfault: `0x4000`～`0xffff`、word境界、MMIOの読書き方向を確認する。
- haltにもfaultにもならない: 分岐先とCALL/RET、ステップ上限、MMCM lockを確認する。
- UART履歴だけ異なる: `UART_TX=0x80000000`、STOREBの下位8 bit、文字順を確認する。
- ILAが見えない: bitとltxの組合せ、JTAGデバイスindex、debug hubクロックを確認する。

RTL変更後は必ず`make test`を先に通し、その後`make hardware-test`を実行する。実機を利用できない
状態ではFPGA関連変更を完了扱いにしない。
