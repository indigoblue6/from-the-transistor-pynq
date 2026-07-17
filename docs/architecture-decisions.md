# 特権・trap基盤の設計判断

## ADR-001: 既存32 bit固定長ISAを維持する

既存opcode `0x00`から`0x17`のエンコードを変更せず、未使用領域
`0x18`から`0x1e`へCSR、trap復帰、待機命令を追加する。既存バイナリとの
後方互換性を優先し、予約bitが非0の命令はreserved encoding trapとする。

## ADR-002: trap状態はCSR、GPR保存はソフトウェアが担当する

ハードウェアはEPC、CAUSE、TVAL、STATUSだけをpreciseに保存する。
trap entry assemblyがSCRATCHとKERNEL_SPを使ってGPRを保存するため、RTLの
状態量を抑えつつ、将来capability registerをtrap frame末尾へ追加できる。

## ADR-003: ECALLのEPCは次命令を指す

ECALLだけはtrap入口で`PC + 4`をEPCへ保存する。handler側の命令長判定を不要にし、
ERETで同じECALLを再実行しない。その他の同期例外は原因命令PCを保存する。

## ADR-004: MMU前段階は独立したベース・リミット保護とする

fetch、load、storeの許可判定を`protection_unit.sv`へ分離する。現在は単一の
`[USER_BASE, USER_LIMIT)`とKernel専用MMIOを検査する。将来はこの判定結果とMMU、
capability checkerの結果を論理積で合成する。

## ADR-005: trap中の再trapはdouble fault停止とする

初期実装ではtrap nestingを行わない。trap handler実行中の同期例外は
unrecoverable double faultとしてCPUを停止し、ILAへ原因、EPC、TVALを保持する。
通常HALT、回復可能trap、double faultは別状態として観測する。

## ADR-006: タイマーの時間単位

RTLの64 bit counterはCPUクロックごとに増加する。参照エミュレータは1 stepを
1 tickとするため、絶対時間ではなくtrap列、UART出力、最終状態を正規化して比較する。
