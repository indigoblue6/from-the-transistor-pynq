; 符号付き条件分岐とループで0～9を出力する
    li r10, 0x80000000
    movi r1, 0
    movi r2, 10
    movi r3, 48
loop:
    add r4, r1, r3
    storeb r4, [r10 + 0]
    addi r1, r1, 1
    blt r1, r2, loop
    movi r4, 10
    storeb r4, [r10 + 0]
    halt
