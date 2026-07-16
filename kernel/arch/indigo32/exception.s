; TrapFrameはr1-r14、元SP、EPC、STATUS、CAUSE、BADADDRの順で76 byte。
trap_vector:
    addi r15, r15, -76
    store r1, [r15 + 0]
    store r2, [r15 + 4]
    store r3, [r15 + 8]
    store r4, [r15 + 12]
    store r5, [r15 + 16]
    store r6, [r15 + 20]
    store r7, [r15 + 24]
    store r8, [r15 + 28]
    store r9, [r15 + 32]
    store r10, [r15 + 36]
    store r11, [r15 + 40]
    store r12, [r15 + 44]
    store r13, [r15 + 48]
    store r14, [r15 + 52]
    addi r1, r15, 76
    store r1, [r15 + 56]
    csrr r1, epc
    store r1, [r15 + 60]
    csrr r1, status
    store r1, [r15 + 64]
    csrr r1, cause
    store r1, [r15 + 68]
    csrr r1, badaddr
    store r1, [r15 + 72]

    add r2, r15, r0
    call trap_handler
    add r15, r1, r0

    load r2, [r15 + 60]
    csrw epc, r2
    load r2, [r15 + 64]
    csrw status, r2
    load r2, [r15 + 4]
    load r3, [r15 + 8]
    load r4, [r15 + 12]
    load r5, [r15 + 16]
    load r6, [r15 + 20]
    load r7, [r15 + 24]
    load r8, [r15 + 28]
    load r9, [r15 + 32]
    load r10, [r15 + 36]
    load r11, [r15 + 40]
    load r12, [r15 + 44]
    load r13, [r15 + 48]
    load r14, [r15 + 52]
    load r1, [r15 + 0]
    load r15, [r15 + 56]
    eret
