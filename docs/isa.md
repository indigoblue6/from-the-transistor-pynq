# 命令セット仕様

本CPUはリトルエンディアン、32 bit固定長、4 byteアラインメントである。演算は2の補数
32 bitでラップアラウンドし、`r0`は常に0である。

## 命令形式

| 形式 | `[31:26]` | `[25:22]` | `[21:18]` | `[17:14]` | 残り |
|---|---|---|---|---|---|
| R | opcode | rd | rs1 | rs2 | `[13:0]=0` |
| I | opcode | rd/データ | rs1/ベース | signed imm18 | なし |
| M | opcode | rd | signed imm22 | - | なし |
| B | opcode | rs1 | rs2 | signed word offset18 | なし |
| J | opcode | signed word offset26 | - | - | なし |

即値は指定幅の2の補数から32 bitへ符号拡張する。LUIだけは`[15:0]`を符号なし16 bit即値、
`[21:16]`を予約0とする。NOP、RET、HALTの`[25:0]`も予約0である。

## opcode

| 値 | 命令 | 形式 | 動作 |
|---:|---|---|---|
| `00` | NOP | - | 何もしない |
| `01`～`08` | ADD, SUB, AND, OR, XOR, SHL, SHR, SAR | R | 算術論理演算 |
| `09` | MOVI | M | 符号拡張imm22を設定 |
| `0a` | ADDI | I | 符号拡張imm18を加算 |
| `0b` | LUI | M特例 | `rd = imm16 << 16` |
| `0c`～`0f` | LOAD, STORE, LOADB, STOREB | I | メモリアクセス |
| `10`～`13` | BEQ, BNE, BLT, BGE | B | レジスタ直接比較分岐 |
| `14` | JMP | J | PC相対ジャンプ |
| `15` | CALL | J | 戻りPCを`r14`へ設定してジャンプ |
| `16` | RET | - | `PC = r14` |
| `17` | HALT | - | リセットまで停止 |

SHL/SHR/SARのシフト量は`rs2[4:0]`である。SHRはゼロ埋め、SARは符号bit埋めである。
BLT/BGEは符号付き比較、BEQ/BNEはbit列比較である。

## PCと分岐

通常の次PCは現在命令アドレス+4である。B/Jの分岐先は
`次PC + (sign_extend(offset) << 2)`で、遅延スロットはない。CALLが`r14`へ保存する値も
次PCである。

## メモリ

実効アドレスは`base + sign_extend(imm18)`のbyteアドレスである。LOAD/STOREは4 byte境界を
要求する。LOADBは1 byteをゼロ拡張し、STOREBはレジスタ下位8 bitだけを書く。

## 擬似命令とディレクティブ

`LI rd,value`はLUIとADDIの2命令へ常に展開する。下位16 bitを符号付き値として扱い、負なら
上位16 bitを1増加するため任意の32 bit値を構築できる。アセンブラは`.word`、`.byte`、
`.org`、`.align`を扱う。

## fault

未定義opcode、予約bit違反、未アラインwordアクセス、ROM外フェッチ、未割当てアドレスは
faultとなる。原因命令はレジスタやメモリを更新せず、リセットまで停止する。HALTも停止するが
正常終了でありfaultとは区別する。
