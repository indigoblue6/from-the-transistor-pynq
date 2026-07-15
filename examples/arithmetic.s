; 40 + 2を計算し、正しければOKを出力する
    li r10, 0x80000000
    movi r2, 40
    movi r3, 2
    add r1, r2, r3
    movi r4, 42
    bne r1, r4, failed
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
