; 例外、割り込み、特権保護をCPU自身で検査する統合テスト。
start:
    li r1, handler
    csrw tvec, r1

    ; ECALLは次命令をEPCへ保存する。
    movi r10, 1
    ecall
after_ecall:
    movi r1, 7
    bne r11, r1, fail
    li r1, after_ecall
    bne r12, r1, fail

    ; 不正命令は原因命令をEPCへ保存し、ハンドラが4進める。
    movi r10, 2
illegal_instruction:
    .word 0xffffffff
after_illegal:
    bne r11, r0, fail
    li r1, after_illegal
    bne r12, r1, fail

    ; 既知opcodeの予約bit違反は専用CAUSE 13とする。
    movi r10, 12
reserved_encoding:
    .word 0x00000001
after_reserved:
    movi r1, 13
    bne r11, r1, fail
    li r1, after_reserved
    bne r12, r1, fail

    ; faultしたLOADは宛先を書き換えない。
    movi r10, 3
    li r1, 0x00004001
    movi r2, 123
load_fault:
    load r2, [r1 + 0]
after_load_fault:
    movi r3, 3
    bne r11, r3, fail
    movi r3, 123
    bne r2, r3, fail
    li r1, 0x00004001
    bne r13, r1, fail

    ; faultしたSTOREはRAMを書き換えない。
    movi r10, 4
    movi r2, 99
store_fault:
    store r2, [r1 + 0]
after_store_fault:
    movi r3, 5
    bne r11, r3, fail
    li r1, 0x00004001
    bne r13, r1, fail

    ; WFI後にtimer割り込みを受理し、次命令へ戻る。
    movi r10, 5
    csrr r1, timer_count_lo
    addi r1, r1, 80
    csrw timer_compare_lo, r1
    movi r1, 0
    csrw timer_compare_hi, r1
    movi r1, 1
    csrw timer_control, r1
    csrw interrupt_enable, r1
    movi r1, 5
    csrw status, r1
    wfi
after_wfi:
    li r1, 0x80000008
    bne r11, r1, fail
    ; CSRSET/CSRCLRは読み書きを一命令で行う。
    movi r1, 3
    csrset interrupt_enable, r1
    movi r1, 1
    csrclr interrupt_enable, r1
    csrr r2, interrupt_enable
    movi r3, 2
    bne r2, r3, fail

    ; 外部割り込みでWFIを解除する。
    movi r10, 8
    movi r1, 2
    csrw interrupt_enable, r1
    movi r1, 5
    csrw status, r1
    wfi
    li r1, 0x80000009
    bne r11, r1, fail
    movi r1, 4
    csrw status, r1

    ; User modeへ移り、CSR書込みとMMIO書込みを拒否する。
    movi r10, 6
    li r1, user_start
    csrw user_base, r1
    csrw epc, r1
    li r1, user_end
    csrw user_limit, r1
    movi r1, 4
    csrw status, r1
    eret

kernel_done:
    movi r1, 0
    li r2, 0x80001000
    store r1, [r2 + 0]
    halt

fail:
    li r3, 0x80000000
    addi r4, r10, 48
    storeb r4, [r3 + 0]
    addi r4, r11, 65
    storeb r4, [r3 + 0]
    movi r1, 1
    li r2, 0x80001000
    store r1, [r2 + 0]
    halt

handler:
    csrr r11, cause
    csrr r12, epc
    csrr r13, badaddr
    movi r1, 1
    beq r10, r1, handler_return
    movi r1, 2
    beq r10, r1, handler_skip
    movi r1, 3
    beq r10, r1, handler_skip
    movi r1, 4
    beq r10, r1, handler_skip
    movi r1, 5
    beq r10, r1, handler_timer
    movi r1, 6
    beq r10, r1, handler_user_csr
    movi r1, 7
    beq r10, r1, handler_user_mmio
    movi r1, 8
    beq r10, r1, handler_external
    movi r1, 9
    beq r10, r1, handler_user_halt
    movi r1, 10
    beq r10, r1, handler_user_wfi
    movi r1, 11
    beq r10, r1, handler_user_ecall
    movi r1, 12
    beq r10, r1, handler_skip
    jmp fail

handler_skip:
    addi r12, r12, 4
    csrw epc, r12
    eret

handler_timer:
    movi r1, -1
    csrw timer_compare_lo, r1
    csrw timer_compare_hi, r1
    eret

handler_user_csr:
    movi r1, 11
    bne r11, r1, fail
    movi r10, 7
    li r1, user_mmio
    csrw epc, r1
    eret

handler_external:
    movi r1, 2
    csrw interrupt_pending, r1
    eret

handler_user_mmio:
    movi r1, 6
    bne r11, r1, fail
    li r1, 0x80000000
    bne r13, r1, fail
    movi r10, 9
    li r1, user_halt
    csrw epc, r1
    eret

handler_user_halt:
    movi r1, 11
    bne r11, r1, fail
    movi r10, 10
    li r1, user_wfi
    csrw epc, r1
    eret

handler_user_wfi:
    movi r1, 11
    bne r11, r1, fail
    movi r10, 11
    li r1, user_ecall
    csrw epc, r1
    eret

handler_user_ecall:
    movi r1, 12
    bne r11, r1, fail
    li r1, user_after_ecall
    bne r12, r1, fail
    li r1, kernel_done
    csrw epc, r1
    movi r1, 12
    csrw status, r1
    eret

handler_return:
    eret

user_start:
    csrw status, r1
user_mmio:
    li r1, 0x80000000
    movi r2, 88
    storeb r2, [r1 + 0]
user_halt:
    halt
user_wfi:
    wfi
user_ecall:
    ecall
user_after_ecall:
user_end:
    nop
