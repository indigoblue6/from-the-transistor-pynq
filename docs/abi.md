# Indigo32 ABI

## 通常関数ABI

r0は定数0、r1は戻り値、r2–r5は第1–第4引数、r6–r10はcaller-saved、r11–r13は
callee-saved、r14はlink register、r15はstack pointerである。stackは下向き、4 byte整列とする。
既存CALL/RETとPynqC ABIは変更しない。

## syscall ABI

Userはr1へ番号、r2–r5へ引数を置いてECALLする。返値は保存frameのr1へ書かれ、負値はerrnoである。
User ECALLのEPCはhardwareが次命令へ進める。

| 番号 | 名称 | 引数 | 戻り値 |
|---:|---|---|---|
| 1 | SYS_WRITE_CHAR | r2=文字 | 0 |
| 2 | SYS_YIELD | なし | 0 |
| 3 | SYS_EXIT | r2=exit code | 復帰しない |
| 4 | SYS_GET_TICKS | なし | timer tick |

UserからUART MMIOを直接操作できない。trap frameは[context-switch.md](context-switch.md)を参照する。
