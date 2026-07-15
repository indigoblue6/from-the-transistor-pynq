# from-the-transistor-pynq

PYNQ-Z1のPLだけで動く独自32 bit CPUから、C-likeコンパイラと自作OSまで積み上げる
教育用プロジェクトである。第1フェーズにはISA、Pythonアセンブラ、Rust参照エミュレータ、
SystemVerilog RTL、共通サンプル、シミュレーションと実機検証環境を含む。

## 必要なツール

- Python 3.10以降とpytest
- Rust 1.85以降とCargo
- GNU Make
- iverilog、Verilator、Vivado xsimのいずれか
- 実機検証にはPYNQ-Z1、Vivado 2025.2以降、USB-JTAG接続

## 基本操作

```bash
make test
make assemble PROGRAM=hello
make emulate PROGRAM=hello
make simulate PROGRAM=hello
```

アセンブラを直接使う場合は次のとおりである。

```bash
python3 assembler/assembler.py examples/hello.s \
  -o build/hello.bin --mem build/hello.mem
cargo run --manifest-path emulator/Cargo.toml -- build/hello.bin --trace
```

`make test`はアセンブラのpytest、Rust単体テスト、4サンプルの出力比較、RTL統合命令テスト、
hello出力比較、不正命令と未アラインアクセスのfaultテストを実行する。RTLシミュレータは
iverilog、Verilator、xsimの順に選ぶ。ただしxsimバッチ経路は現時点で未検証である。

## 実機テスト

FPGA関連変更はシミュレーションだけでは完了としない。PYNQ-Z1をJTAG接続して実行する。

```bash
make hardware-test
# 外付けUSB-UARTも同時検証する場合
make hardware-uart-test PORT=/dev/ttyUSB2
```

このターゲットはhelloをアセンブルし、XC7Z020用bitstreamを生成し、書込み後にILAをhaltで
トリガする。ILAにはfaultと送信器が受理した直近17 byteのUART履歴も含まれる。物理UART TXはPMODA JA1、
115200 baud、8N1である。LED0点灯は正常HALT、LED1点灯はfaultを示す。

## サンプル

| プログラム | 期待出力 |
|---|---|
| `hello.s` | `Hello, PYNQ CPU!` |
| `arithmetic.s` | `OK` |
| `branch.s` | `0123456789` |
| `call.s` | `CALL OK` |

## 設計文書

- `docs/isa.md`: 命令エンコードと実行意味
- `docs/architecture.md`: データパス、FSM、RTL構成
- `docs/abi.md`: 呼出し規約とスタック
- `docs/memory-map.md`: ROM、RAM、MMIO
- `docs/hardware-debugging.md`: PYNQ-Z1実機デバッグ手順
- `docs/hardware-test-results.md`: 実機テスト記録

## 現在の制限

乗除算、浮動小数点、割り込み、特権レベル、MMU、キャッシュ、AXI、DDR、PS連携はない。
5個目以降の引数、構造体ABI、リンク可能オブジェクト形式も未定義である。例外は原因レジスタを
持たずfault状態で停止する。実機CPUクロックは31.25 MHzである。

## ロードマップ

次フェーズでは字句解析・構文解析、型検査、ABI準拠コード生成、スタックフレーム、リンカを
実装する。その後、割り込み・タイマー・GPIO、特権モード、ブートROM、OSのスケジューラ、
システムコール、メモリアロケータ、ファイルシステムへ進む。
