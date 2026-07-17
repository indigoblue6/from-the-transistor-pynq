# Trap frameとコンテキスト切替

hardwareはEPC/CAUSE/TVAL/STATUSだけをCSRへ保存する。`kernel/scheduler_demo.s`の入口はSCRATCHで
r1を一時退避し、KERNEL_SPでUser stackからKernel専用stackへ切り替えてからGPRを保存する。

| offset | 内容 |
|---:|---|
| 0–52 | r1–r14（4 byte刻み） |
| 56 | trap前r15/SP |
| 60 | EPC |
| 64 | STATUS |
| 68 | CAUSE |
| 72 | TVAL |

frameは76 byteで、r0は保存しない。将来はversion/sizeヘッダとcapability register save areaを
後置できる。通常ABIはr1=返値、r2–r5=引数、r11–r13=callee-saved、r14=LR、r15=SPを維持する。
trapは通常関数境界ではないため全可変GPRを保存する。各taskは独立したkernel trap frame、User SP、
EPC、STATUS、state、exit codeを持つ。demoのround-robinはtimerごとにA→B→fault task→Aを選び、
fault task終了後はA/Bだけを継続する。
