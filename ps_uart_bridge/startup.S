.section .vectors, "ax"
.global _start
_start:
    cpsid if
    cps #0x13
    /* Linuxの変換表を使わず、物理アドレスでPS周辺へアクセスする。 */
    mrc p15, 0, r0, c1, c0, 0
    bic r0, r0, #1
    bic r0, r0, #4
    bic r0, r0, #0x1000
    mcr p15, 0, r0, c1, c0, 0
    dsb
    isb
    ldr sp, =0x0003fff0
    bl main
1:
    b 1b
