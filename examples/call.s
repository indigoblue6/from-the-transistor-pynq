; 初期ABIに従いadd関数を呼び出す
    li r10, 0x80000000
    movi r2, 40
    movi r3, 2
    call add
    movi r4, 42
    bne r1, r4, failed
    movi r1, 67
    storeb r1, [r10 + 0]
    movi r1, 65
    storeb r1, [r10 + 0]
    movi r1, 76
    storeb r1, [r10 + 0]
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
