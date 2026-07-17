# Trap仕様

## precise trap

同期例外は原因命令のPCをEPCへ保存し、その命令のGPR、CSR、メモリ副作用をcommitしない。
ECALLだけは再実行を避けるためEPCへ次命令PCを保存する。割り込みは命令境界で受理し、EPCは
未実行の次命令を指す。入口はCAUSE/TVAL/EPC/STATUSを確定し、Kernel modeでTVECへ移る。
TVEC=0、またはERET前にさらに同期trapが起きた場合はdouble faultとしてunrecoverable FAULTへ停止する。
正常HALT、recoverable trap、double faultを別状態・デバッグ信号で区別する。

CAUSE bit31は非同期割り込みを表す。同期原因は次のとおりである。

| code | 原因 |
|---:|---|
| 0 | illegal instruction |
| 1 | instruction address misaligned |
| 2 | instruction access fault |
| 3 | load address misaligned |
| 4 | load access fault |
| 5 | store address misaligned |
| 6 | store access fault |
| 7 | Kernel ECALL |
| 11 | privilege violation |
| 12 | User ECALL |
| 13 | reserved encoding violation |
| 14 | breakpoint（予約） |
| 15 | capability fault（予約） |
| 16 | instruction page fault（予約） |
| 17 | load page fault（予約） |
| 18 | store page fault（予約） |

割り込みは`0x80000008`がtimer、`0x80000009`がexternal/UART、`0x8000000a`がsoftwareである。
TVAL（旧名BADADDR）は不正命令word、問題のアドレス、または特権違反命令wordを保持する。
