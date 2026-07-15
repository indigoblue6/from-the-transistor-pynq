# CPUアーキテクチャ

## 全体構成

本CPUは32 bit固定長ISAを実行する、正しさと観測性を優先したマルチサイクルCPUである。
命令ROMとデータRAMを分離したHarvard構成とし、両方を同期Block RAMで実装する。
PYNQ-Z1実機では125 MHz入力をMMCMで31.25 MHzへ変換してCPUを動かす。

```text
命令BRAM → CPU FSM → レジスタファイル
              │          ↕
              ├──────→ ALU
              │
              └──────→ データBRAM / MMIO → UART
```

## データパス

レジスタファイルは16本の32 bitレジスタを持ち、3読出し・1書込みである。第3読出しは
STOREの書込み値と分岐比較のために用いる。`r0`読出しは常に0で、書込みは無視する。
ALUは加減算、論理演算、3種類のシフトを担当する。シフト量は右オペランドの下位5 bitである。
分岐比較はCPU制御部で行い、BLT/BGEだけを符号付き比較とする。

## 制御ステートマシン

| 状態 | 動作 |
|---|---|
| FETCH | PCの範囲とアラインメントを検査し、同期ROM読出しを開始 |
| FETCH_WAIT | ROM出力を命令レジスタへ格納し、PCを4増加 |
| DECODE | opcodeと予約bitを検査 |
| EXECUTE | ALU、分岐、ジャンプ、アドレス計算を実行 |
| MEMORY_REQUEST | 同期RAMまたはMMIOへ要求を提示 |
| MEMORY_WAIT | 読出し値またはfaultを受理 |
| HALTED | リセットまで停止 |
| FAULT | リセットまで停止 |

ALU命令は5状態相当、メモリ命令は7状態相当を通る。CALL時点のPCは既に次命令を指すため、
その値を`r14`へ書く。メモリfault時はロード先レジスタもメモリも更新しない。

## RTLモジュール

| ファイル | 役割 |
|---|---|
| `cpu.sv` | PC、命令レジスタ、マルチサイクルFSM |
| `decoder.sv` | フィールド抽出、符号拡張、予約bit検査 |
| `alu.sv` | 算術論理演算 |
| `register_file.sv` | 16本のレジスタと`r0`保護 |
| `memory_map_true_bram.sv` | 同期命令ROM、4 byte laneデータRAM、MMIO |
| `uart_fifo.sv` | MMIO連続書込みを吸収する32 byte FIFO |
| `uart_tx.sv` | 合成可能な8N1 UART送信器 |
| `pynq_z1_top.sv` | MMCM、実機端子、ILA観測信号 |

リセットは同期アクティブHighである。実機トップではMMCM lock後もリセットカウンタを保持し、
クロック安定後に解除する。UART TXはPMODA JA1へ115200 baudで出力する。

## デバッグ

CPUはPC、命令、状態、レジスタ書込みを外部へ出す。実機トップはhalt、fault、直近17文字の
UART履歴をILAへ接続する。LED0はhalt、LED1はfault、LED2はUART ready、LED3はheartbeatである。

## 割り込みの将来方針

第1フェーズに割り込みはない。将来は例外PCと状態を保存する専用CSR、割り込みベクタ、
割り込み許可bitを追加し、命令境界でのみ受付ける。ABIでは割り込み入口が全caller-saved
レジスタを保存し、専用の例外復帰命令でPCと許可状態を復元する方針とする。
