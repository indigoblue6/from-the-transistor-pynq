# PynqC最小ランタイム

## 起動と終了

コンパイラはアセンブリ先頭へ`_start`を生成する。SPを`0x00010000`へ設定し、データRAMの
グローバル、ゼロ領域、NUL終端文字列を初期化して`main`を呼ぶ。`main`の`r1`を
`SIM_EXIT(0x80001000)`へword書込みし、その後HALTする。

Harvard構成では命令ROMの定数をLOADできないため、現段階のグローバル初期化は命令列で行う。
`runtime/crt0.s`は境界説明用で、実際の`_start`は配置アドレスを知るコンパイラが生成する。

## UARTと文字列

- `putchar(char)`: `UART_TX`へ下位8 bitを書き込む。
- `puts(char*)`: NULまでbyte loadし、改行を自動追加しない。
- `print_int(int)`: 0、負号、10進数字を出力する。
- `strlen(char*)`: NULまでのbyte数を返す。
- `memset` / `memcpy`: byte単位で指定サイズを処理し、先頭ポインタを返す。
- `panic(char*)`: メッセージ出力後、SIM_EXITへ1を書いて停止する。

UART_STATUSはCPU仕様上常に送信可能である。実機ではMMIO後段のFIFOが物理UART速度差を吸収する。

## 算術補助関数

`__mulsi3`はshift-add、`__divsi3`と`__modsi3`は符号を分離した反復減算で実装する。結果は
32 bitラップする。ゼロ除算はSIM_EXIT=1で停止する。`INT_MIN / -1`など絶対値を符号付きで
表せない境界は現ランタイムの制限であり、将来unsigned long divisionへ置き換える。

組み込み関数の型はコンパイラへ登録され、PynqCソースで宣言する必要はない。`runtime/*.pc`は
APIを人間が確認するための宣言ファイルで、プリプロセッサや複数ファイルリンクには依存しない。
