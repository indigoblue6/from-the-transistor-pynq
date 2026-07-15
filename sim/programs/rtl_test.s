; RTLの通常命令を網羅する自己検査プログラム
    li r10, 0x80000000
    li r9, 0x00004000

    movi r1, 0x55aa
    store r1, [r9 + 0]
    load r2, [r9 + 0]
    bne r1, r2, failed
    movi r3, 0xab
    storeb r3, [r9 + 1]
    loadb r4, [r9 + 1]
    bne r3, r4, failed
    load r4, [r9 + 0]
    movi r5, 0xabaa
    bne r4, r5, failed

    movi r1, 12
    movi r2, 10
    add r3, r1, r2
    movi r4, 22
    bne r3, r4, failed
    sub r3, r1, r2
    movi r4, 2
    bne r3, r4, failed
    and r3, r1, r2
    movi r4, 8
    bne r3, r4, failed
    or r3, r1, r2
    movi r4, 14
    bne r3, r4, failed
    xor r3, r1, r2
    movi r4, 6
    bne r3, r4, failed

    movi r1, 1
    movi r2, 3
    shl r3, r1, r2
    movi r4, 8
    bne r3, r4, failed
    shr r3, r3, r2
    bne r3, r1, failed
    movi r1, -8
    movi r2, 2
    sar r3, r1, r2
    movi r4, -2
    bne r3, r4, failed

    beq r1, r1, beq_ok
    jmp failed
beq_ok:
    bne r1, r2, bne_ok
    jmp failed
bne_ok:
    blt r1, r2, blt_ok
    jmp failed
blt_ok:
    bge r2, r1, bge_ok
    jmp failed
bge_ok:
    jmp jmp_ok
    jmp failed
jmp_ok:
    movi r2, 40
    movi r3, 2
    call add
    movi r4, 42
    bne r1, r4, failed
    movi r0, 7
    bne r0, r11, failed

    movi r1, 84
    storeb r1, [r10 + 0]
    movi r1, 69
    storeb r1, [r10 + 0]
    movi r1, 83
    storeb r1, [r10 + 0]
    movi r1, 84
    storeb r1, [r10 + 0]
    movi r1, 32
    storeb r1, [r10 + 0]
    movi r1, 79
    storeb r1, [r10 + 0]
    movi r1, 75
    storeb r1, [r10 + 0]
    movi r1, 10
    storeb r1, [r10 + 0]
    halt

failed:
    movi r1, 70
    storeb r1, [r10 + 0]
    halt

add:
    add r1, r2, r3
    ret
