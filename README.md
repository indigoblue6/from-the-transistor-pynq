# from-the-transistor-pynq

## Kernel/User・trap・scheduler milestone

Indigo32は既存ISAを保ったままKernel/User mode、32bit CSR、precise recoverable trap、64bit timer、
external IRQ、ECALL/ERET/WFI、base-limit保護を備える。`kernel/scheduler_demo.s`はUser task A/Bを
timerでpreemptし、syscall経由でUARTへ出力する。第3taskのKernel領域storeはtaskだけを終了し、A/Bは継続する。

```bash
make test-privilege
make test-traps
make test-interrupts
make test-scheduler
make emulate-kernel
make simulate-kernel
make test-all
# PYNQ-Z1実機
make hardware-kernel-test PORT=/dev/ttyUSB1
# minicomを開いたまま対話表示
make hardware-kernel-console
```

代表出力は`KERNEL BOOT`、`USER MODE OK`、A/B、`TRAP cause=STORE_ACCESS_FAULT`、
`BAD TASK KILLED`、fault後のA/B、`SCHEDULER OK`である。参照機は1 step/tick、RTLは1 clock/tickなので
量子内の連続A/B数は異なり、differential testはその長さだけを正規化する。MMU、AXI DDR、Linux、
capability register/tagged memoryは未実装である。


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
既定のコンソールはDigilent Adept内蔵PS UART (`/dev/ttyUSB1`) とし、PL mailboxをCortex-A9のbridgeが転送する。

```bash
make hardware-test
# 外付けUSB-UARTも同時検証する場合
make hardware-uart-test PORT=/dev/ttyUSB2
```

このターゲットはhelloをアセンブルし、XC7Z020用bitstreamを生成し、書込み後にILAをhaltで
トリガする。ILAにはfaultと送信器が受理した直近17 byteのUART履歴も含まれる。代替のPL直結UART TXはPMODA JA1、
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

hardware乗除算、浮動小数点、MMU、キャッシュ、AXI、DDRはない。特権・trap・timer・外部IRQは実装済みである。
5個目以降の引数、構造体ABI、リンク可能オブジェクト形式も未定義である。同期例外はCAUSE/EPC/TVALを保存するrecoverable trapであり、trap中の再例外だけはdouble fault停止する。
実機CPUクロックは31.25 MHzである。

## ロードマップ

次フェーズではPynqCへIR、オブジェクト形式、リンカ、複数翻訳単位を追加する。並行して
割り込み・タイマー・GPIO、特権モード、ブートROM、OSのスケジューラ、システムコール、
メモリアロケータ、ファイルシステムへ進む。

## PynqCコンパイラ

第2フェーズではRust製C-likeコンパイラ`pynqc`と最小ランタイムを提供する。

```bash
make compiler
make compile PROGRAM=hello
make run-c PROGRAM=hello
make simulate-c PROGRAM=hello
make test-c-integration
make test-c-rtl
make test-all
```

`pynqc`はLexer、Parser、Span付きAST、名前解決、型検査、独自ISAコード生成を行う。
ローカル／グローバル変数、制御文、4引数までの関数と再帰、ポインタ、1次元配列、
文字列、`sizeof`、短絡論理、ソフトウェア乗除算に対応する。実機PynqC検証は次で行う。

```bash
make hardware-c-test
```

言語の詳細は`docs/language-spec.md`、生成ABIは`docs/compiler-abi.md`、ランタイムは
`docs/runtime.md`を参照する。
