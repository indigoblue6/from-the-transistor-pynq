# 割り込みとタイマー

64bit `TIMER_COUNT`はRTLではCPUクロックごと、参照エミュレータでは`step()`ごとに1増える。
`TIMER_CONTROL.bit0`、`INTERRUPT_ENABLE.bit0`、`STATUS.IE`が有効で、countがcompare以上なら
pendingとなる。compareを将来値へ更新すると解除される。trap中はIE=0なので再入しない。

pending/enable bitは0=timer、1=external（UART RXを含む）、2=softwareである。CPUコアの
`external_irq`はラッチされ、`INTERRUPT_PENDING`へbit1を書いてW1Cする。UART RXはlevel入力なので
FIFOが空になると解除される。優先順位はtimer、external、softwareである。WFIはKernel専用で、
enabled interruptを受けるまでFSMの待機状態に留まる。クロックゲーティングは行わない。
