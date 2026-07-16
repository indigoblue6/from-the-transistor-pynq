# Indigo32特権・保護モデル

## 実行モード

リセット直後はKernel modeである。Kernel modeは全命令、命令ROM、データRAM、MMIOへアクセス
できる。User modeではCSR書込み、`ERET`、割り込み制御およびMMIOへの直接アクセスを禁止する。

## ベース・リミット保護

IndigoOS 0.1はMMUを持たず、`USER_BASE`以上`USER_LIMIT`未満の単一連続領域だけをUser modeへ
許可する。命令fetch、LOAD、STOREの全byteが範囲内でなければaccess faultとなる。加算時の
32 bit桁あふれも範囲外として扱う。`0x80000000`以上のMMIOはUSER_LIMITに関係なく拒否する。

Harvard BRAMの制約から、初期ユーザーイメージは命令ROM後半とデータRAM前半を含む連続スロットへ
静的配置する。カーネルは命令ROM前半とデータRAM後半を利用する。複数ユーザータスクの完全な
相互分離には複数組のコード／データ境界またはDDR導入が必要であり、現版の既知の制限である。

## ユーザーポインタ

syscallは`pointer >= USER_BASE`、`length >= 0`、`pointer + length`が桁あふれせず
`<= USER_LIMIT`であることを検証する。NUL終端文字列には別途最大長を設ける。検証前の参照、
カーネルポインタの返却、無制限な文字列走査は禁止する。

## 既知の限界

ページング、仮想アドレス、実行／読出し／書込み権限の分離、共有メモリ、DMA保護はない。
単一ベース・リミット方式のため、同じ物理スロットを共有するタスク間の隔離もない。これらは
AXI/DDRとMMU導入時に拡張する。

